import dash
from dash import dcc, html, Input, Output, State, dash_table
from dash.exceptions import PreventUpdate
import plotly.express as px
import plotly.graph_objects as go
import pandas as pd
from pynmeagps import NMEAReader, NMEAParseError, NMEAMessage
from pyrtcm import RTCMReader, RTCMParseError # Optional for RTCM
app = dash.Dash(__name__, prevent_initial_callbacks="initial_duplicate")
app.layout = html.Div([
    html.H1("NMEA Parser and Visualizer"),
    dcc.Textarea(
        id='nmea-input',
        value='''$GBGSV,3,1,10,11,42,295,21,12,60,029,20,21,33,133,29,22,30,070,17,1*79
$GBGSV,3,2,10,24,23,200,15,25,35,264,21,34,61,325,18,43,,,18,1*46
$GBGSV,3,3,10,44,43,065,14,50,,,23,1*42
$GBGSV,2,1,08,21,33,133,17,22,30,070,13,23,14,313,17,24,23,200,15,5*79
$GBGSV,2,2,08,25,35,264,19,34,61,325,23,43,,,17,44,43,065,17,5*45
$GQGSV,1,1,00,1*65
$GQGSV,1,1,00,8*6C
$GNGGA,072507.000,3546.325706,N,07841.331701,W,1,27,0.61,113.332,M,-33.017,M,,*43
$GNRMC,072507.000,A,3546.325706,N,07841.331701,W,0.008,235.00,290925,,,A,V*2A
$GNGLL,3546.325706,N,07841.331701,W,072507.000,A,A*5E
$GNVTG,235.00,T,,M,0.008,N,0.015,K,A*2B
$GNGSA,A,3,06,11,12,14,17,19,21,22,24,,,,1.10,0.61,0.92,1*06
$GNGSA,A,3,76,86,87,,,,,,,,,,1.10,0.61,0.92,2*0E
$GNGSA,A,3,13,21,23,26,33,31,,,,,,,1.10,0.61,0.92,3*09
$GNGSA,A,3,11,12,21,22,24,25,34,44,23,,,,1.10,0.61,0.92,4*0F
$GNGSA,A,3,,,,,,,,,,,,,1.10,0.61,0.92,5*09
$GPGSV,3,1,10,06,65,009,35,11,55,271,30,12,35,309,18,14,12,159,23,1*66
$GPGSV,3,2,10,17,39,094,13,19,58,071,20,21,32,188,25,22,31,164,19,1*6B
$GPGSV,3,3,10,24,05,256,16,25,,,16,1*50
$GPGSV,1,1,04,06,65,009,26,11,55,271,31,14,12,159,16,21,32,188,16,8*6F
$GLGSV,1,1,03,76,49,238,20,86,53,358,29,87,36,294,19,1*4C
$GAGSV,2,1,05,13,13,051,11,21,25,157,18,23,69,210,31,26,62,024,33,7*76
$GAGSV,2,2,05,33,54,263,30,7*43
$GAGSV,1,1,04,23,69,210,28,26,62,024,18,31,41,309,15,33,54,263,24,1*70
$GBGSV,3,1,10,11,42,295,22,12,60,029,20,21,33,133,31,22,30,070,17,1*73
$GBGSV,3,2,10,24,23,200,14,25,35,264,22,34,61,325,19,43,,,19,1*44
$GBGSV,3,3,10,44,43,065,16,50,,,26,1*45
$GBGSV,2,1,08,21,33,133,18,22,30,070,16,23,14,313,17,24,23,200,17,5*71
$GBGSV,2,2,08,25,35,264,20,34,61,325,25,43,,,17,44,43,065,17,5*49
$GQGSV,1,1,00,1*65
$GQGSV,1,1,00,8*6C
$GNGGA,072508.000,3546.325707,N,07841.331701,W,1,27,0.61,113.332,M,-33.017,M,,*4D
$GNRMC,072508.000,A,3546.325707,N,07841.331701,W,0.005,235.00,290925,,,A,V*29
$GNGLL,3546.325707,N,07841.331701,W,072508.000,A,A*50
$GNVTG,235.00,T,,M,0.005,N,0.010,K,A*23
$GNGSA,A,3,06,11,12,14,17,19,21,22,24,,,,1.12,0.61,0.94,1*02
$GNGSA,A,3,76,86,87,,,,,,,,,,1.12,0.61,0.94,2*0A
$GNGSA,A,3,13,21,23,26,33,31,,,,,,,1.12,0.61,0.94,3*0D''', # Your sample data pre-loaded
        style={'width': '100%', 'height': 300}
    ),
    html.Button('Parse and Display', id='parse-button', n_clicks=0),
    html.Div([
        html.Label('Select Constellations:'),
        dcc.Checklist(id='const-select', inline=True)
    ]),
    html.Div(id='parsed-output'),
    dcc.Graph(id='horizontal-plot', style={'width': '50%'}),
    dcc.Graph(id='cno-time-plot'),
    html.Div(id='cno-stats-table')
])
def generate_horizontal(selected_const, latest_ep, colors):
    sat_list = []
    for (const, prn), d in latest_ep['sat_dict'].items():
        if const not in selected_const:
            continue
        active = prn in latest_ep.get('active', {}).get(const, [])
        sat_list.append({
            'const': const,
            'prn': prn,
            'cno': d['cno'] or 0,
            'az': d['az'] or '',
            'elv': d['elv'] or '',
            'active': active
        })
    sat_df = pd.DataFrame(sat_list)
    if sat_df.empty:
        return go.Figure()
    sat_df['az'] = pd.to_numeric(sat_df['az'], errors='coerce')
    sat_df['elv'] = pd.to_numeric(sat_df['elv'], errors='coerce')
    sat_df.sort_values('az', ascending=True, na_position='last', inplace=True)
    fig = go.Figure()
    for _, row in sat_df.iterrows():
        fig.add_trace(go.Bar(
            x=[f"{row['const']}-{row['prn']:02d}"],
            y=[row['cno']],
            name=row['const'],
            marker=dict(
                color=colors.get(row['const'], 'gray'),
                opacity=1 if row['active'] else 0.3
            ),
            text=row['cno'] if row['cno'] > 0 else '',
            textposition='outside',
            customdata=[[row['az'], row['elv']]],
            hovertemplate='Constellation: %{name}<br>Satellite: %{x}<br>C/N0: %{y} dB-Hz<br>Azimuth: %{customdata[0]}<br>Elevation: %{customdata[1]}'
        ))
    fig.update_layout(
        title='GNSS Signal View - Horizontal View',
        xaxis_title='Satellite',
        yaxis_title='C/N0 (dB-Hz)',
        showlegend=True,
        bargap=0.1,
        bargroupgap=0.1
    )
    return fig
@app.callback(
    [Output('parsed-output', 'children'),
     Output('cno-time-plot', 'figure'),
     Output('cno-stats-table', 'children'),
     Output('const-select', 'options'),
     Output('const-select', 'value'),
     Output('horizontal-plot', 'figure', allow_duplicate=True)],
    Input('parse-button', 'n_clicks'),
    State('nmea-input', 'value')
)
def parse_nmea(n_clicks, nmea_data):
    if n_clicks == 0:
        raise PreventUpdate
    if not nmea_data:
        return "No data provided.", px.line(), html.Div(), [], [], go.Figure()
    lines = nmea_data.strip().split('\n')
    epochs = []
    pending_sat_dict = {}
    pending_active = {}
    constellation_map = {
        'GP': 'GPS', 'GL': 'GLONASS', 'GA': 'Galileo',
        'GB': 'BeiDou', 'GQ': 'QZSS', 'GN': 'Multi'
    }
    system_id_map = {
        1: 'GPS', 2: 'GLONASS', 3: 'Galileo',
        4: 'BeiDou', 5: 'QZSS'
    }
    colors = {
        'GPS': 'blue', 'GLONASS': 'cyan', 'Galileo': 'green',
        'BeiDou': 'red', 'QZSS': 'purple', 'SBAS': 'blue',
        'NAVIC': 'yellow', 'Unknown': 'gray'
    }
    for line in lines:
        line = line.strip()
        if not line:
            continue
        try:
            if line.startswith('$'):
                msg = NMEAReader.parse(line)
            else:
                msg = RTCMReader.parse(bytes.fromhex(line))
            if hasattr(msg, 'identity') and 'RTCM' in msg.identity.upper():
                # Handle RTCM if needed
                pass
            else:
                if msg.msgID == 'GSV':
                    constellation = constellation_map.get(msg.talker, 'Unknown')
                    for i in range(1, 5):
                        svid = getattr(msg, f'svid_{i:02d}', None)
                        if svid:
                            key = (constellation, svid)
                            pending_sat_dict[key] = {
                                'cno': getattr(msg, f'cno_{i:02d}', None),
                                'elv': getattr(msg, f'elv_{i:02d}', None),
                                'az': getattr(msg, f'az_{i:02d}', None)
                            }
                elif msg.msgID == 'GSA':
                    sys_id = getattr(msg, 'systemId', None) or int(getattr(msg, 'gnssID', 0))
                    if sys_id:
                        const = system_id_map.get(sys_id, 'Unknown')
                        prns = [getattr(msg, f'sv_{i:02d}', None) for i in range(1, 13) if getattr(msg, f'sv_{i:02d}', None)]
                        pending_active[const] = prns
                elif msg.msgID == 'GGA':
                    time_obj = msg.time
                    time_sec = (time_obj.hour * 3600 + time_obj.minute * 60 + time_obj.second + time_obj.microsecond / 1e6)
                    position = {
                        'lat': msg.lat,
                        'lon': msg.lon,
                        'alt': msg.alt
                    }
                    epoch = {
                        'time_obj': time_obj,
                        'time_sec': time_sec,
                        'position': position,
                        'sat_dict': pending_sat_dict.copy(),
                        'active': pending_active.copy()
                    }
                    epochs.append(epoch)
                    pending_sat_dict = {}
                    pending_active = {}
        except (NMEAParseError, RTCMParseError) as e:
            print(f"Parse error: {e}")
    if not epochs:
        return "No position data found.", px.line(), html.Div(), [], [], go.Figure()
    min_sec = min(ep['time_sec'] for ep in epochs)
    # Latest position
    latest_ep = epochs[-1]
    position = latest_ep['position']
    output = [
        html.H3("Latest Position"),
        html.Pre(f"Latitude: {position['lat']}\nLongitude: {position['lon']}\nAltitude: {position['alt']} m")
    ]
    # All constellations
    all_const = set()
    for ep in epochs:
        for const, _ in ep['sat_dict']:
            all_const.add(const)
    options = [{'label': c, 'value': c} for c in sorted(all_const)]
    value = sorted(all_const)
    # Horizontal view
    horizontal_fig = generate_horizontal(value, latest_ep, colors)
    # C/N0 time series
    data_list = []
    for ep in epochs:
        for (const, prn), d in ep['sat_dict'].items():
            if d['cno'] is not None:
                data_list.append({
                    'time': ep['time_sec'] - min_sec,
                    'const': const,
                    'prn': prn,
                    'cno': d['cno']
                })
    df = pd.DataFrame(data_list)
    cno_fig = go.Figure()
    for const in df['const'].unique():
        const_df = df[df['const'] == const]
        for prn in const_df['prn'].unique():
            sat_df = const_df[const_df['prn'] == prn]
            cno_fig.add_trace(go.Scatter(
                x=sat_df['time'],
                y=sat_df['cno'],
                mode='lines+markers',
                name=f"{const}-{prn:02d}",
                line=dict(color=colors.get(const, 'gray'))
            ))
    cno_fig.update_layout(
        title='C/N0 Over Time per Satellite',
        xaxis_title='Time (seconds)',
        yaxis_title='C/N0 (dB-Hz)',
        showlegend=True
    )
    # Stats table
    if not df.empty:
        df['sat_id'] = df['const'] + '-' + df['prn'].astype(str).str.zfill(2)
        stats = df.groupby('sat_id')['cno'].agg(['mean', 'max', 'min']).reset_index()
        stats_table = dash_table.DataTable(
            data=stats.to_dict('records'),
            columns=[
                {'name': 'Signal ID', 'id': 'sat_id'},
                {'name': 'AVG', 'id': 'mean'},
                {'name': 'MAX', 'id': 'max'},
                {'name': 'MIN', 'id': 'min'}
            ]
        )
    else:
        stats_table = html.Div()
    return output, cno_fig, stats_table, options, value, horizontal_fig
@app.callback(
    Output('horizontal-plot', 'figure', allow_duplicate=True),
    Input('const-select', 'value'),
    State('parse-button', 'n_clicks'),
    State('nmea-input', 'value')
)
def update_horizontal(selected_const, n_clicks, nmea_data):
    if not selected_const or n_clicks == 0:
        return go.Figure()
    # Re-parse to get latest_ep (inefficient, but since static data, ok; alternatively use Store)
    # For brevity, re-run parsing logic here (duplicate code, but simple)
    lines = nmea_data.strip().split('\n')
    epochs = []
    pending_sat_dict = {}
    pending_active = {}
    constellation_map = {
        'GP': 'GPS', 'GL': 'GLONASS', 'GA': 'Galileo',
        'GB': 'BeiDou', 'GQ': 'QZSS', 'GN': 'Multi'
    }
    system_id_map = {
        1: 'GPS', 2: 'GLONASS', 3: 'Galileo',
        4: 'BeiDou', 5: 'QZSS'
    }
    colors = {
        'GPS': 'blue', 'GLONASS': 'cyan', 'Galileo': 'green',
        'BeiDou': 'red', 'QZSS': 'purple', 'SBAS': 'blue',
        'NAVIC': 'yellow', 'Unknown': 'gray'
    }
    for line in lines:
        line = line.strip()
        if not line:
            continue
        try:
            if line.startswith('$'):
                msg = NMEAReader.parse(line)
            else:
                msg = RTCMReader.parse(bytes.fromhex(line))
            if hasattr(msg, 'identity') and 'RTCM' in msg.identity.upper():
                pass
            else:
                if msg.msgID == 'GSV':
                    constellation = constellation_map.get(msg.talker, 'Unknown')
                    for i in range(1, 5):
                        svid = getattr(msg, f'svid_{i:02d}', None)
                        if svid:
                            key = (constellation, svid)
                            pending_sat_dict[key] = {
                                'cno': getattr(msg, f'cno_{i:02d}', None),
                                'elv': getattr(msg, f'elv_{i:02d}', None),
                                'az': getattr(msg, f'az_{i:02d}', None)
                            }
                elif msg.msgID == 'GSA':
                    sys_id = getattr(msg, 'systemId', None) or int(getattr(msg, 'gnssID', 0))
                    if sys_id:
                        const = system_id_map.get(sys_id, 'Unknown')
                        prns = [getattr(msg, f'sv_{i:02d}', None) for i in range(1, 13) if getattr(msg, f'sv_{i:02d}', None)]
                        pending_active[const] = prns
                elif msg.msgID == 'GGA':
                    time_obj = msg.time
                    time_sec = (time_obj.hour * 3600 + time_obj.minute * 60 + time_obj.second + time_obj.microsecond / 1e6)
                    position = {
                        'lat': msg.lat,
                        'lon': msg.lon,
                        'alt': msg.alt
                    }
                    epoch = {
                        'time_obj': time_obj,
                        'time_sec': time_sec,
                        'position': position,
                        'sat_dict': pending_sat_dict.copy(),
                        'active': pending_active.copy()
                    }
                    epochs.append(epoch)
                    pending_sat_dict = {}
                    pending_active = {}
        except (NMEAParseError, RTCMParseError) as e:
            print(f"Parse error: {e}")
    if not epochs:
        return go.Figure()
    latest_ep = epochs[-1]
    return generate_horizontal(selected_const, latest_ep, colors)
if __name__ == '__main__':
    app.run(debug=True)
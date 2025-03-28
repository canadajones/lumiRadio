# This is the lumiRadio Liquidsoap configuration file
#
# Usually you shouldn't need to edit this file, because most
# configuration is done via environment variables.

# Load settings from environment
settings.init.allow_root := true
host = environment.get(default="ice", "ICECAST_HOST")
port =
  int_of_string(default=8000, environment.get(default="8000", "ICECAST_PORT"))
user = environment.get(default="source", "ICECAST_USER")
password = environment.get(default="hackme", "ICECAST_PASSWORD")
mount = environment.get(default="lumiradio", "ICECAST_MOUNT")
langley_url =
  environment.get(default="http://langley:8000/played", "LANGLEY_URL")

# Log to stdout
log.stdout.set(true)

# Allow telnet access for requesting songs
# settings.server.telnet.set(true)
# settings.server.telnet.port.set(1234)
# settings.server.telnet.bind_addr.set("0.0.0.0")
settings.server.timeout.set(-1.0)
settings.server.socket.set(true)
settings.server.socket.path.set("/usr/share/liquidsoap/lumiradio.sock")
settings.server.socket.permissions.set(0o666)

# Define interactive variables
v = interactive.float("volume", 1.)

# Persist them
interactive.persistent("/usr/share/liquidsoap/vars.json")

# server.telnet()
server.harbor()
interactive.harbor()

# Load music directory
playlist = playlist(reload_mode="watch", "/music/playlist.m3u")

# Apply ReplayGain
playlist = replaygain(playlist)

# Skip blanks
playlist = mksafe(blank.eat(playlist))

srq = request.queue(id="srq")
prioq = request.queue(id="prioq")

# Play priority queue first, then the regular song request queue and then the playlist
radio = fallback([prioq, srq, playlist])

# Set volume
radio = amplify(v, radio)

current_track = ref([])
def on_track(t) =
  current_track.set(t)
  headers = [("Content-Type", "application/json")]
  payload = json()
  payload.add("artist", t["artist"])
  payload.add("title", t["title"])
  payload.add("album", t["album"])
  payload.add("filename", t["filename"])
  payload_s = json.stringify(payload)

  print(payload_s)
  _res = http.post(data=payload_s, headers=headers, langley_url)
  ()
end
radio = source.on_track(radio, on_track)

def on_current_track(cli) =
  t = current_track()
  json.stringify(t)
end
server.register(
  usage="current_track",
  description=
    "Show the currently playing track",
  "current_track",
  on_current_track
)

def on_song_request_queue(cli) =
  prioq = prioq.queue()
  prioq_length = list.length(prioq)
  srq = srq.queue()
  srq_length = list.length(srq)

  json_list = ref([])
  for i = 0 to
    prioq_length - 1
  do
    elem = list.nth(prioq, i)
    obj = json()
    request.read_metadata(elem)
    elem_meta = request.metadata(elem)
    obj.add("artist", elem_meta["artist"])
    obj.add("title", elem_meta["title"])
    obj.add("album", elem_meta["album"])
    obj.add("filename", elem_meta["filename"])
    obj.add("queue", "prioq")
    print(json.stringify(obj))
    json_list := list.add(obj, json_list())
  end
  for i = 0 to
    srq_length - 1
  do
    elem = list.nth(srq, i)
    request.read_metadata(elem)
    obj = json()
    elem_meta = request.metadata(elem)
    obj.add("artist", elem_meta["artist"])
    obj.add("title", elem_meta["title"])
    obj.add("album", elem_meta["album"])
    obj.add("filename", elem_meta["filename"])
    obj.add("queue", "srq")
    print(json.stringify(obj))
    json_list := list.add(obj, json_list())
  end

  json.stringify(json_list())
end
server.register(
  usage="song_request_queue",
  description=
    "Show the song request queue",
  "song_request_queue",
  on_song_request_queue
)

# Output to Icecast
output.icecast(
  %mp3(bitrate = 128),
  id="lumiradio",
  user=user,
  host=host,
  port=port,
  password=password,
  mount=mount,
  send_icy_metadata=true,
  radio
)

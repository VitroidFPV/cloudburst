# Cloudburst

Cloudburst is an opinionated desktop experience for a technical user who wants to manage torrents without a dense control surface.

## Language

**Torrent**:
A managed BitTorrent item in any state, including downloading, seeding, complete, paused, or errored.
_Avoid_: Download, transfer, job

**Transfer activity**:
The current upload or download activity associated with a torrent.
_Avoid_: Torrent, when referring specifically to current network activity

**qBittorrent instance**:
A running qBittorrent application that Cloudburst can control.
_Avoid_: Server, daemon, torrent backend

**Connection profile**:
A saved description of how Cloudburst reaches and authenticates to a qBittorrent instance.
_Avoid_: Server, account

**Connection resolution**:
The process of selecting a reachable qBittorrent instance, either through known local information or a manual connection profile.
_Avoid_: Automatic discovery

**Active connection profile**:
The one saved connection profile Cloudburst is currently using. Other profiles may be retained, but their torrents are not combined with the active profile.
_Avoid_: Current server, selected account

**Disconnected**:
The state in which the active qBittorrent instance cannot be reached and its last known torrents are stale.
_Avoid_: Empty, offline mode

**Remove torrent**:
Stop managing a torrent while retaining its downloaded content.
_Avoid_: Delete torrent

**Remove torrent and files**:
Stop managing a torrent and delete its downloaded content.
_Avoid_: Delete, remove data

**Add torrent**:
Start managing one or more torrents by submitting magnet links, URLs, or .torrent files to the active qBittorrent instance.
_Avoid_: Import, upload, create torrent

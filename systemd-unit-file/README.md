# PAS systemd unit file

A systemd unit file is the startup and shutdown script for systemd managed servers.

Do carefully inspect the unit file, and only proceed if you understand the settings.
The most prominent setting is the working directory (WorkingDirectory), which is where PAS will generate its archives.

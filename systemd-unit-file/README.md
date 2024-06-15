# PAS systemd unit file

A systemd unit file is the startup and shutdown script for systemd managed servers.

Do carefully inspect the unit file, and only proceed if you understand the settings.
The most prominent setting is the working directory (WorkingDirectory), which is where PAS will generate its archives.

# Database connection

PAS connects to a database, and therefore must be configured to be able to connect to the intended database.
By default PAS connects to the default postgres socket in /tmp, with the default port number 4321.

If you want to connect to a different socket location, or to a TCP address, set it using -c in the unit file.

Example:

```
pas -c "postgres://?host=127.0.0.1&port=1000&user=admin&dbname=mydatabase&sslrootcert=/postgres/ca.pem&sslkey=/postgres/tls_key.key&sslcert=/postgres/tls_cert.crt"
```

# Monitoring multiple postgres clusters

If you want to monitor more than one postgres cluster, create MULTIPLE unit files named "pas-mydatabase.service", each with their unique connection specified in the unit.
Also important is to have each of them use a DIFFERENT working directory, currently the archive names do not specify the source database from which the data originated.

# Installation

1. Copy the unit/service file to `/etc/systemd/system/`, optionally change the unit file name from `pas.service` to `pas-mydatabase.service`.
2. Reload systemd: `systemctlt daemon-reload` (this is safe and does not interrupt any processes).
3. Set the unit to be started at boot: `systemctl enable pas`. (if you want to only run pas at specific times, do not run "enable")
4. Start the unit: `systemctl start pas`.

# Warning!

PAS currently does not clean up it's archive files, so that is a task that has to be executed independently from PAS.
With lots of database usage, PAS archives can grow big. Please monitor carefully.

# Removal

1. Stop the service if running: `systemctl stop pas`.
2. Remove the unit: `systemctl disable pas`.
3. Remove the unit file: `rm /etc/systemd/system/pas.service`.
4. Reload systemd: `systemctl daemon-reload`.

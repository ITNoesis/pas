# Connecting to postgres
The connections that PAS does are created using an SQLX PgPool connectionpool, currently of 1 connection.
Because PAS is supposed to connect locally, it's strongly advised to use a socket connection.

By default the connection string is: 'postgresql:///', which means connecting to the postgres database via socket in /tmp called ".s.PGSQL.5432".

If your socket is in another place, and your databases uses a different port number, use the following socket specification:
"postgresql://<DATABASE>?host=<SOCKET DIRECTORY>&port=<PORT>&user=<USER>"

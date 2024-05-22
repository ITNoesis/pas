# PAS: PostgreSQL Activity Statistics
A utility to extract activity statistics from a (local) postgres database cluster at high speed for troubleshooting purposes.

## Active Session History
In order to get an overview of session activity, PAS can sample the active sessions and record the state of these. 
This gives an overview of ON-CPU or waiting state, which can be used to determine if a database cluster is CPU, IO, lock or otherwise bound.
![Active sessions](/Images/active_sessions.png)

## Active Session History by Query ID
Of course a server wide overview is nice to see where the time is spent, but you also would want to see it per query. 
![Active sessions by query id](/Images/active_session_by_query_id.png)

## IO Latency
A database is sensitive to IO latency, so IO latencies is something that is important to see.
![IO latency](/Images/io_latencies.png)

## IO bandwidth
A database can produce a high amount of IO, and it's interesting to see where that IO comes from.
Another nice view here is to see what are the block write reasons: in postgres, a backend might need to write if the checkpointer and background writer cannot write enough buffers quickly enough.
![IO bandwidth](/Images/io_bandwidth.png)

## Transaction ID age
An important concept of PostgreSQL is the transaction ID, for which it's a good thing to have an overview of the transaction and multi-transaction ID ages.
![Transaction ID age](/Images/transaction_id_age.png)

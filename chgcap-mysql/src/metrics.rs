// /**
//  * The last record processing time, which is updated after {@link MySqlSourceReader} fetches a
//  * batch of data. It's mainly used to report metrics sourceIdleTime for sourceIdleTime =
//  * System.currentTimeMillis() - processTime.
//  */
// private volatile long processTime = 0L;

// /**
//  * currentFetchEventTimeLag = FetchTime - messageTimestamp, where the FetchTime is the time the
//  * record fetched into the source operator.
//  */
// private volatile long fetchDelay = 0L;

// /**
//  * emitDelay = EmitTime - messageTimestamp, where the EmitTime is the time the record leaves the
//  * source operator.
//  */
// private volatile long emitDelay = 0L;

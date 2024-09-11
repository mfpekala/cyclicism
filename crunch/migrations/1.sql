CREATE TABLE IF NOT EXISTS contemporary_multimedia (
    uri TEXT NOT NULL PRIMARY KEY,
    url TEXT NOT NULL,
    rank INTEGER NOT NULL,
    format TEXT,
    type_ TEXT NOT NULL,
    subtype TEXT,
    caption TEXT
);

CREATE TABLE IF NOT EXISTS contemporary_article (
    uri TEXT NOT NULL PRIMARY KEY,
    url TEXT NOT NULL,
    yy INTEGER NOT NULL,
    mm INTEGER NOT NULL,
    dd INTEGER NOT NULL,
    title TEXT NOT NULL,
    abstract TEXT NOT NULL,
    section TEXT NOT NULL,
    subsection TEXT NOT NULL,
    item_type TEXT NOT NULL,
    kicker TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS current (
    uri TEXT NOT NULL PRIMARY KEY,
    rank INTEGER NOT NULL,
    time_added TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS combos (
    contemporary_uri TEXT NOT NULL,
    past_uri TEXT NOT NULL,
    score FLOAT NOT NULL,
    PRIMARY KEY (contemporary_uri, past_uri)
);

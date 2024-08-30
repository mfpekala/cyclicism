CREATE TABLE IF NOT EXISTS scraped_headline (
    uri TEXT NOT NULL PRIMARY KEY,
    main TEXT NOT NULL,
    kicker TEXT,
    content_kicker TEXT,
    print_headline TEXT NOT NULL,
    name TEXT,
    seo TEXT,
    sub TEXT
);

CREATE TABLE IF NOT EXISTS scraped_multimedia (
    uri TEXT NOT NULL PRIMARY KEY,
    rank INTEGER NOT NULL,
    subtype TEXT NOT NULL,
    caption TEXT,
    credit TEXT,
    type_ TEXT NOT NULL,
    url TEXT NOT NULL,
    height INTEGER NOT NULL,
    width INTEGER NOT NULL,
    crop_name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS scraped_article (
    uri TEXT NOT NULL PRIMARY KEY,
    web_url TEXT NOT NULL,
    snippet TEXT NOT NULL,
    print_page TEXT,
    print_section TEXT,
    source TEXT NOT NULL,
    pub_date TEXT NOT NULL,
    document_type TEXT NOT NULL,
    news_desk TEXT NOT NULL,
    section_name TEXT NOT NULL,
    type_of_material TEXT NOT NULL
);

use chrono::Datelike;
use sqlx::{postgres::PgPoolOptions, PgPool, Pool, Postgres, Row};

use crate::nyt::{
    clean_snippet, parse_pub_date, ContemporaryArticle, FrontendArticle, FrontendImage,
    ScrapedArticle,
};

pub async fn get_pg_pool(max_connections: u32) -> anyhow::Result<Pool<Postgres>> {
    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .connect("postgres://postgres:password@localhost:5432/cyclicism")
        .await?;
    Ok(pool)
}

pub async fn apply_migrations(pool: &Pool<Postgres>) -> anyhow::Result<()> {
    let mut files = std::fs::read_dir("migrations")?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    files.sort_by_key(|entry| {
        let path = entry.path();
        let file_stem = path.file_stem().unwrap().to_str().unwrap();
        file_stem.parse::<u32>().unwrap()
    });

    for entry in files {
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "sql" {
                let contents = tokio::fs::read_to_string(path).await?;
                for part in contents.split(";") {
                    sqlx::query(part).execute(pool).await?;
                }
            }
        }
    }

    Ok(())
}

impl ScrapedArticle {
    pub async fn upsert(&self, conn: &PgPool) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO scraped_article (
                uri, web_url, snippet, print_page, print_section, source, pub_date, document_type, news_desk, section_name, type_of_material
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11
            )
            ON CONFLICT (uri) DO UPDATE
            SET web_url = $2, snippet = $3, print_page = $4, print_section = $5, source = $6, pub_date = $7, document_type = $8, news_desk = $9, section_name = $10, type_of_material = $11
            "#,
        )
        .bind(self.uri.as_str())
        .bind(self.web_url.as_str())
        .bind(self.snippet.as_str())
        .bind(self.print_page.as_ref().map(|s| s.as_str()))
        .bind(self.print_section.as_ref().map(|s| s.as_str()))
        .bind(self.source.as_str())
        .bind(self.pub_date.as_str())
        .bind(self.document_type.as_str())
        .bind(self.news_desk.as_str())
        .bind(self.section_name.as_str())
        .bind(self.type_of_material.as_str())
        .execute(conn)
        .await?;
        sqlx::query(
            r#"
            INSERT INTO scraped_headline (
                uri, main, kicker, content_kicker, print_headline, name, seo, sub
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8
            )
            ON CONFLICT (uri) DO UPDATE
            SET main = $2, kicker = $3, content_kicker = $4, print_headline = $5, name = $6, seo = $7, sub = $8
            "#,
        ).bind(self.uri.as_str())
        .bind(self.headline.main.as_str())
        .bind(self.headline.kicker.as_ref().map(|s| s.as_str()))
        .bind(self.headline.content_kicker.as_ref().map(|s| s.as_str()))
        .bind(self.headline.print_headline.as_str())
        .bind(self.headline.name.as_ref().map(|s| s.as_str()))
        .bind(self.headline.seo.as_ref().map(|s| s.as_str()))
        .bind(self.headline.sub.as_ref().map(|s| s.as_str()))
        .execute(conn)
        .await?;
        if let Some(first_media) = self.multimedia.iter().next() {
            sqlx::query(
                r#"
                INSERT INTO scraped_multimedia (
                    uri, rank, subtype, caption, credit, type_, url, height, width, crop_name
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10
                )
                ON CONFLICT (uri) DO UPDATE
                SET rank = $2, subtype = $3, caption = $4, credit = $5, type_ = $6, url = $7, height = $8, width = $9, crop_name = $10
                "#,
            )
            .bind(self.uri.as_str())
            .bind(first_media.rank as i32)
            .bind(first_media.subtype.as_str())
            .bind(first_media.caption.as_ref().map(|s| s.as_str()))
            .bind(first_media.credit.as_ref().map(|s| s.as_str()))
            .bind(first_media.type_.as_str())
            .bind(first_media.url.as_str())
            .bind(first_media.height as i32)
            .bind(first_media.width as i32)
            .bind(first_media.crop_name.as_str())
            .execute(conn)
            .await?;
        }
        Ok(())
    }
}

impl FrontendArticle {
    pub async fn from_uri(uri: &str, pg: &PgPool) -> anyhow::Result<Self> {
        let Some(article_row) = sqlx::query(
            r#"
            SELECT web_url, snippet, print_section, pub_date, document_type, news_desk, type_of_material
            FROM scraped_article
            WHERE uri = $1
            "#,
        )
        .bind(uri)
        .fetch_optional(pg)
        .await?
        else {
            return Err(anyhow::anyhow!("Couldn't select article row"));
        };

        let Some(headline_row) = sqlx::query(
            r#"
                SELECT main
                FROM scraped_headline
                WHERE uri = $1
                "#,
        )
        .bind(uri)
        .fetch_optional(pg)
        .await?
        else {
            return Err(anyhow::anyhow!("Couldn't select headline row"));
        };

        let multimedia = sqlx::query(
            r#"
                    SELECT url, caption
                    FROM scraped_multimedia
                    WHERE uri = $1
                    "#,
        )
        .bind(uri)
        .fetch_optional(pg)
        .await?;

        let image = multimedia.map(|m| FrontendImage {
            url: m.get(0),
            caption: m.get(1),
        });

        let pub_date: String = article_row.get(3);
        let naive_date = parse_pub_date(&pub_date);

        Ok(FrontendArticle {
            uri: uri.to_string(),
            web_url: article_row.get(0),
            headline_main: headline_row.get(0),
            snippet: clean_snippet(article_row.get(1)),
            year: naive_date.year_ce().1 as u32,
            month: naive_date.month0() + 1,
            day: naive_date.day(),
            image,
            print_section: article_row.get(2),
            document_type: article_row.get(4),
            news_desk: article_row.get(5),
            type_of_material: article_row.get(6),
        })
    }
}

impl ContemporaryArticle {
    pub async fn upsert(&self, pg: &Pool<Postgres>) -> anyhow::Result<()> {
        let (yy, mm, dd) = self.get_date_parts()?;
        sqlx::query(
            r#"
            INSERT INTO contemporary_article
                (uri, url, yy, mm, dd, title, abstract, section, subsection, item_type, kicker)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (uri) DO UPDATE
            SET url = $2, yy = $3, mm = $4, dd = $5, title = $6, abstract = $7, section = $8, subsection = $9, item_type = $10, kicker = $11
            "#,
        )
        .bind(&self.uri)
        .bind(&self.url)
        .bind(yy as i32)
        .bind(mm as i32)
        .bind(dd as i32)
        .bind(&self.title)
        .bind(&self.abstract_)
        .bind(&self.section)
        .bind(&self.subsection)
        .bind(&self.item_type)
        .bind(&self.kicker)
        .execute(pg)
        .await?;
        if let Some(multi) = self.multimedia.as_ref() {
            if let Some(first) = multi.iter().next() {
                sqlx::query(
                    r#"
                INSERT INTO contemporary_multimedia
                    (uri, url, rank, format, type_, subtype, caption)
                VALUES
                    ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT (uri) DO UPDATE
                SET url = $2, rank = $3, format = $4, type_ = $5, subtype = $6, caption = $7
                "#,
                )
                .bind(&self.uri)
                .bind(&first.url)
                .bind(0)
                .bind(&first.format)
                .bind(&first.type_)
                .bind(&first.subtype)
                .bind(&first.caption)
                .execute(pg)
                .await?;
            }
        }
        Ok(())
    }
}

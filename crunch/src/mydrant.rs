use chrono::Datelike;
use fastembed::EmbeddingModel;
use qdrant_client::{
    qdrant::{
        CreateCollectionBuilder, Distance, OptimizersConfigDiffBuilder, PointStruct, PointsIdsList,
        QueryPointsBuilder, SetPayloadPointsBuilder, UpdateCollectionBuilder, UpsertPointsBuilder,
        VectorParamsBuilder,
    },
    Payload, Qdrant,
};
use uuid::Uuid;

use crate::nyt::{parse_pub_date, uri_to_uuid, ScrapedArticle};

/// Identifies what part of the article should do the embedding
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub enum BedSource {
    HeadlineMain,
}

/// Information that should be attached to all point structs to allow for interesting filtering
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommonInfo {
    pub uri: String,
    pub year: u32,
    pub month: u32,
    pub day: u32,
    pub print_section: Option<String>,
    pub document_type: String,
    pub news_desk: String,
    pub type_of_material: String,
}
impl Into<Payload> for CommonInfo {
    fn into(self) -> Payload {
        let json_string = serde_json::to_string(&self).unwrap();
        serde_json::from_str(&json_string).unwrap()
    }
}
pub fn break_article_for_mydrant(
    article: ScrapedArticle,
    source: BedSource,
) -> Option<(Uuid, String, CommonInfo)> {
    let naive_date = parse_pub_date(&article.pub_date);
    match source {
        BedSource::HeadlineMain => Some((
            uri_to_uuid(&article.uri),
            article.headline.main,
            CommonInfo {
                uri: article.uri,
                year: naive_date.year() as u32,
                month: naive_date.month0() + 1,
                day: naive_date.day(),
                print_section: article.print_section,
                document_type: article.document_type,
                news_desk: article.news_desk,
                type_of_material: article.type_of_material,
            },
        )),
    }
}

// An embedding along with the info needed to put it in qdrant
pub struct DetailedEmbedding {
    pub uuid: Uuid,
    pub bed: Vec<f32>,
    pub info: CommonInfo,
}

pub struct Collection {
    /// What part of the article are we embedding? (e.g headline, snippet...)
    source: BedSource,
    bed_dim: u64, // TODO: Figure out how to infer this from fastembed (why is it not obvious?)
    /// Which model was used to do this embedding?
    model: EmbeddingModel,
    /// What kind of distance metric to use on vectors?
    distance: Distance,
    client: Qdrant,
}
impl Collection {
    pub fn new(
        source: BedSource,
        bed_dim: u64,
        model: EmbeddingModel,
        distance: Distance,
        client: Qdrant,
    ) -> Self {
        Self {
            source,
            bed_dim,
            model,
            distance,
            client,
        }
    }

    fn collection_name(&self) -> String {
        format!("{:?}___{:?}___{:?}", self.source, self.model, self.distance)
    }

    pub async fn ensure_created(&self) -> anyhow::Result<()> {
        if !self
            .client
            .collection_exists(self.collection_name())
            .await?
        {
            self.client
                .create_collection(
                    CreateCollectionBuilder::new(self.collection_name())
                        .vectors_config(VectorParamsBuilder::new(self.bed_dim, self.distance)),
                )
                .await?;
        }
        self.client
            .update_collection(
                UpdateCollectionBuilder::new(self.collection_name())
                    .optimizers_config(OptimizersConfigDiffBuilder::default()),
            )
            .await?;
        Ok(())
    }

    pub async fn upsert(&self, data: Vec<DetailedEmbedding>) -> anyhow::Result<()> {
        let points = data
            .into_iter()
            .map(|details| PointStruct::new(details.uuid.to_string(), details.bed, details.info))
            .collect::<Vec<_>>();
        self.client
            .upsert_points(UpsertPointsBuilder::new(self.collection_name(), points).wait(true))
            .await?;
        Ok(())
    }

    pub async fn overwrite_payload(&self, uuid: Uuid, info: CommonInfo) -> anyhow::Result<()> {
        let payload: Payload = info.into();
        self.client
            .overwrite_payload(
                SetPayloadPointsBuilder::new(self.collection_name(), payload)
                    .points_selector(PointsIdsList {
                        ids: vec![uuid.to_string().into()],
                    })
                    .wait(true),
            )
            .await?;
        Ok(())
    }

    pub async fn unfuck_grey_status(&self) -> anyhow::Result<()> {
        self.client
            .update_collection(
                UpdateCollectionBuilder::new(self.collection_name())
                    .optimizers_config(OptimizersConfigDiffBuilder::default()),
            )
            .await?;
        Ok(())
    }

    pub async fn top_k(&self, bed: Vec<f32>, k: u64) -> anyhow::Result<Vec<CommonInfo>> {
        if bed.len() as u64 != self.bed_dim {
            return Err(anyhow::anyhow!(
                "bed is not the right size, got {}, expected {}",
                bed.len(),
                self.bed_dim
            ));
        }
        let res = self
            .client
            .query(
                QueryPointsBuilder::new(self.collection_name())
                    .query(bed)
                    .limit(k)
                    .with_payload(true),
            )
            .await?;
        Ok(res
            .result
            .into_iter()
            .map(|p| {
                let serde_string = serde_json::to_string(&p.payload).unwrap();
                let info = serde_json::from_str::<CommonInfo>(&serde_string);
                info
            })
            .filter_map(|info| info.ok())
            .collect())
    }
}

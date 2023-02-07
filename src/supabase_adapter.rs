use serde::Deserialize;
use async_recursion::async_recursion;


#[allow(non_snake_case)]
#[derive(Deserialize, Debug, Default)]
pub struct SupabaseCommandsList {
    page: usize,
    perPage: usize,
    totalItems: usize,
    items: Vec<SupabaseCommandItem>
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
pub struct SupabaseCommandItem {
    collectionId: String,
    collectionName: String,
    command: String,
    created: String,
    id: String,
    sound: String,
    updated: String
}

pub async fn get_list(api: &String, collection: &str, page: usize, per_page: usize) -> Result<SupabaseCommandsList, reqwest::Error> {
    reqwest::get(api.to_owned() + &format!("collections/{collection}/records?page={page}&perPage={per_page}"))
        .await?
        .json::<SupabaseCommandsList>()
        .await
}

pub async fn get_full_list(api: &String, collection: &str) -> Result<SupabaseCommandsList, reqwest::Error> {
    let mut res = request(api, collection, SupabaseCommandsList::default(), 1).await?;

    res.page = 1;
    res.totalItems = res.items.len();
    res.perPage = res.items.len();

    Ok(res)
}

#[async_recursion]
async fn request(api: &String, collection: &str, mut res: SupabaseCommandsList, page: usize) -> Result<SupabaseCommandsList, reqwest::Error> {
    let mut current = get_list(api, collection, page, 100).await?;

    res.items.append(&mut current.items);

    if res.items.len() < res.totalItems {
        return request(api, collection, res, page + 1).await;
    }

    Ok(res)
}

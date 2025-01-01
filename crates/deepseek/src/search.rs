use crate::api;

const DETERMINE_PROMPT: &'static str = "Please first think deeply about if you are 100% sure about the definition of every word in the query (instead of guessing or \"seems to be\"), and then answer whether it would be beneficial to search the web in order to get a better understanding about the query or check your answer's correctness or not if you were to answer this query very accurately without any possible problem. Do not answer the original query. If it is beneficial to search the web, answer \"yes\" (or \"no\" if not) without extra characters.";
const TERM_PROMPT: &'static str = "Provide a google search term based on search query provided below in less than 20 words";
const SUMMARY_PROMPT: &'static str = "You are an AI assistant tasked with summarizing content relevant to '{}'. Please provide a concise summary.";
const FINAL_PROMPT: &'static str = "The user provides a bunch of search results for search query {search_term}. \n{content}\nBased on on the search results provided by the user, provide a response to user's query. PLEASE ANSWER THE QUERY IN THE SAME LANGUAGE THAT IT'S ASKED!";
const SEARCH_API_HOST_BASE_URL: &'static str = "http://127.0.0.1:5000";

/// deepseek-bot's search api format
#[derive(serde::Deserialize)]
pub struct SearchResults {
    pub articles: Vec<String>,
}

async fn search(query: String, client: reqwest::Client) -> Result<Vec<String>, Box<dyn std::error::Error + Sync + Send>> {
    let response = client.get(format!("{}/search?query={}", SEARCH_API_HOST_BASE_URL, query.replace("\"", ""))).send().await?;
    let payload = serde_json::from_str::<SearchResults>(response.text().await?.as_str())?;
    Ok(payload.articles)
}


/// ref: https://cookbook.openai.com/examples/third_party/web_search_with_google_api_bring_your_own_browser_tool
pub struct SearchDriver {
    pub api: api::DeepSeekAPI,
}

impl SearchDriver {
    pub fn from(api: api::DeepSeekAPI) -> Self {
        Self { api }
    }
    pub async fn determine(&self, query: String) -> Result<bool, Box<dyn std::error::Error + Sync + Send>> {
        let res = self.api.single_message_dialog_with_system(20, query, String::from(DETERMINE_PROMPT)).await?.trim().to_string();
        Ok(res != "no")
    }
    pub async fn generate_search_term(&self, query: String) -> Result<String, Box<dyn std::error::Error + Sync + Send>> {
        self.api.single_message_dialog_with_system(20, query, TERM_PROMPT.to_string()).await
    }
    /// Returns a system prompt
    pub async fn search_and_summary(&self, query: String) -> Result<String, Box<dyn std::error::Error + Sync + Send>> {
        let term = self.generate_search_term(query.to_owned()).await?;
        let articles = search(term.to_owned(), self.api.client.clone()).await?;
        let mut summarized_content = String::new();
        for (index, article) in articles.into_iter().enumerate() {
            if let Ok(summary) = self.api.single_message_dialog_with_system(100, article, SUMMARY_PROMPT.replace("{}", term.to_owned().as_str())).await {
                summarized_content.push_str(format!("Search order: {index}\nSummary: {summary}\n--------------------------------------------------------------------------------\n").as_str());
            }
        }
        Ok(FINAL_PROMPT
            .replace("{search_term}", &term)
            .replace("{content}", &summarized_content)
        )
    }
}

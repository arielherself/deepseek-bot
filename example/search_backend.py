import requests
from bs4 import BeautifulSoup
from flask import Flask, request, jsonify
from search_engine_parser.core.engines.duckduckgo import Search
import asyncio
from urllib.parse import urlparse, parse_qs, unquote

app = Flask(__name__)

def clean_duckduckgo_link(link):
    """Remove DuckDuckGo redirection from the link."""
    parsed_url = urlparse(link)
    if parsed_url.netloc == 'duckduckgo.com' and parsed_url.path == '/l/':
        # Extract the 'uddg' query parameter
        query_params = parse_qs(parsed_url.query)
        if 'uddg' in query_params:
            # Decode the URL
            return unquote(query_params['uddg'][0])
    return link

def retrieve_content(url, max_tokens=7000):
    print(f'Fetching: {url}')
    for _ in range(2):
        try:
            headers = {'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/109.0.0.0 Safari/537.36'}
            response = requests.get(url, headers=headers, timeout=5)
            response.raise_for_status()

            soup = BeautifulSoup(response.content, 'html.parser')
            for script_or_style in soup(['script', 'style']):
                script_or_style.decompose()

            text = soup.get_text(separator=' ', strip=True)
            characters = max_tokens * 4  # Approximate conversion
            text = text[:characters]
            return text
        except requests.exceptions.RequestException as e:
            print(f"Failed to retrieve {url}: {e}")
    return ''

def run_async_search(query):
    # Create a new event loop for the search
    print(f'Searching {query}')
    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    try:
        # Perform the search
        results = Search().search(query)
        # Clean up DuckDuckGo links
        cleaned_links = [clean_duckduckgo_link(link) for link in results['links']]
        # print(list(map(retrieve_content, cleaned_links[:5])))
        return list(map(retrieve_content, cleaned_links[:5]))
    finally:
        loop.close()

@app.route('/search', methods=['GET'])
def search():
    # Get the query parameter from the request
    query = request.args.get('query')

    if not query:
        return jsonify({"error": "Query parameter is required"}), 400

    try:
        # Run the search in a synchronous context
        links = run_async_search(query)
        # Return the links as a JSON object
        return jsonify({"articles": links})
    except Exception as e:
        return jsonify({"error": str(e)}), 500

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=5000)

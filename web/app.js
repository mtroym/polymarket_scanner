document.addEventListener('DOMContentLoaded', () => {
    const marketsList = document.getElementById('markets-list');
    const marketCount = document.getElementById('market-count');
    const lastUpdated = document.getElementById('last-updated');
    const refreshBtn = document.getElementById('refresh-btn');
    const searchInput = document.getElementById('search');

    let allMarkets = [];

    async function fetchMarkets() {
        try {
            refreshBtn.textContent = 'Loading...';
            refreshBtn.disabled = true;
            
            // Assuming the server root is the project root, so data is at ../data/markets.json
            // But usually web servers serve from the web root. 
            // If user runs `python3 -m http.server` in project root, then `/data/markets.json` works.
            // If user runs in `web/`, then `../data` is not accessible via HTTP usually.
            // We will assume the server is started at project root.
            const response = await fetch('/data/markets.json');
            
            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }
            
            const data = await response.json();
            
            // data.markets is an object/map, convert to array
            allMarkets = Object.values(data.markets);
            
            // Sort by volume (descending) if available, otherwise random/default
            allMarkets.sort((a, b) => {
                const volA = parseFloat(a.volume || 0);
                const volB = parseFloat(b.volume || 0);
                return volB - volA;
            });

            renderMarkets(allMarkets);
            updateStats();
            
        } catch (error) {
            console.error('Error fetching markets:', error);
            marketsList.innerHTML = `
                <div class="loading" style="color: var(--danger-color)">
                    Error loading data: ${error.message}<br>
                    <small>Make sure you are running a local server at the project root (e.g., <code>python3 -m http.server</code>)</small>
                </div>
            `;
        } finally {
            refreshBtn.textContent = 'Refresh Data';
            refreshBtn.disabled = false;
        }
    }

    function renderMarkets(markets) {
        marketsList.innerHTML = '';
        
        if (markets.length === 0) {
            marketsList.innerHTML = '<div class="loading">No markets found</div>';
            return;
        }

        markets.forEach(market => {
            const card = document.createElement('div');
            card.className = 'market-card';
            
            let outcomesHtml = '';
            try {
                const outcomes = JSON.parse(market.outcomes);
                const prices = market.outcomePrices ? JSON.parse(market.outcomePrices) : [];
                
                outcomesHtml = '<div class="outcomes-list">';
                outcomes.forEach((outcome, index) => {
                    const price = prices[index] !== undefined ? prices[index] : 'N/A';
                    // Format price nicely
                    let displayPrice = price;
                    if (typeof price === 'number') {
                        displayPrice = (price * 100).toFixed(1) + '%';
                    } else if (price !== 'N/A') {
                         displayPrice = (parseFloat(price) * 100).toFixed(1) + '%';
                    }

                    outcomesHtml += `
                        <div class="outcome-item">
                            <span class="outcome-name">${outcome}</span>
                            <span class="outcome-price">${displayPrice}</span>
                        </div>
                    `;
                });
                outcomesHtml += '</div>';
            } catch (e) {
                console.warn('Error parsing outcomes for market:', market.question, e);
                outcomesHtml = '<div class="outcomes-list">Error parsing outcomes</div>';
            }

            const volume = market.volume ? `$${parseFloat(market.volume).toLocaleString()}` : 'N/A';
            const liquidity = market.liquidity ? `$${parseFloat(market.liquidity).toLocaleString()}` : 'N/A';

            card.innerHTML = `
                <h3 class="market-title" title="${market.question}">${market.question}</h3>
                ${outcomesHtml}
                <div class="market-meta">
                    <span>Vol: ${volume}</span>
                    <span>Liq: ${liquidity}</span>
                </div>
            `;
            
            marketsList.appendChild(card);
        });
    }

    function updateStats() {
        marketCount.textContent = allMarkets.length;
        lastUpdated.textContent = new Date().toLocaleTimeString();
    }

    // Search functionality
    searchInput.addEventListener('input', (e) => {
        const term = e.target.value.toLowerCase();
        const filtered = allMarkets.filter(m => 
            m.question.toLowerCase().includes(term) || 
            (m.description && m.description.toLowerCase().includes(term))
        );
        renderMarkets(filtered);
    });

    refreshBtn.addEventListener('click', fetchMarkets);

    // Initial load
    fetchMarkets();
});

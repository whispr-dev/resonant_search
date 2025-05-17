// static_templates/app.js

document.addEventListener('DOMContentLoaded', () => {
    const searchForm = document.getElementById('search-form');
    const queryInput = document.getElementById('query');
    const resultsSection = document.getElementById('results-section');
    const searchResults = document.getElementById('search-results');
    const resultStats = document.getElementById('result-stats');
    const quantumScoring = document.getElementById('quantum-scoring');
    const persistenceScoring = document.getElementById('persistence-scoring');

    // Handle search form submission
    searchForm.addEventListener('submit', async (e) => {
        e.preventDefault();
        
        const query = queryInput.value.trim();
        if (!query) return;
        
        // Show loading state
        searchResults.innerHTML = '<div class="loading">Searching quantum resonance space...</div>';
        resultsSection.style.display = 'block';
        
        try {
            // Build search URL with parameters
            const useQuantum = quantumScoring.checked;
            const usePersistence = persistenceScoring.checked;
            
            const url = new URL('/api/search', window.location.origin);
            url.searchParams.append('q', query);
            url.searchParams.append('quantum', useQuantum ? '1' : '0');
            url.searchParams.append('persistence', usePersistence ? '1' : '0');
            url.searchParams.append('limit', '15');
            
            // Fetch search results
            const response = await fetch(url);
            
            if (!response.ok) {
                throw new Error(`Search failed: ${response.statusText}`);
            }
            
            const data = await response.json();
            displayResults(data);
            
            // Update URL to make results shareable
            const shareUrl = new URL(window.location);
            shareUrl.searchParams.set('q', query);
            if (useQuantum) shareUrl.searchParams.set('quantum', '1');
            if (usePersistence) shareUrl.searchParams.set('persistence', '1');
            window.history.pushState({}, '', shareUrl);
            
        } catch (error) {
            searchResults.innerHTML = `
                <div class="error-message">
                    <h3>Search Error</h3>
                    <p>${error.message || 'An unexpected error occurred'}</p>
                </div>
            `;
            console.error('Search error:', error);
        }
    });

    // Display search results in the UI
    function displayResults(data) {
        // Update stats
        resultStats.textContent = `Found ${data.results.length} results (${data.elapsed_ms}ms)`;
        
        // Clear previous results
        searchResults.innerHTML = '';
        
        if (data.results.length === 0) {
            searchResults.innerHTML = `
                <div class="no-results">
                    <h3>No results found</h3>
                    <p>Try different search terms or adjust your search options.</p>
                </div>
            `;
            return;
        }
        
        // Add each result to the page
        data.results.forEach(result => {
            const resultCard = document.createElement('div');
            resultCard.className = 'result-card';
            
            // Format scores for display
            const formatScore = (score) => score ? score.toFixed(4) : 'N/A';
            
            resultCard.innerHTML = `
                <h3><a href="${result.url}" target="_blank">${escapeHtml(result.title)}</a></h3>
                <a href="${result.url}" class="url" target="_blank">${escapeHtml(result.url)}</a>
                <p class="snippet">${escapeHtml(result.snippet)}</p>
                <div class="scores">
                    <div class="score">
                        <span class="label">Resonance:</span>
                        <span class="value">${formatScore(result.score)}</span>
                    </div>
                    ${result.quantum_score ? `
                        <div class="score">
                            <span class="label">Quantum:</span>
                            <span class="value">${formatScore(result.quantum_score)}</span>
                        </div>
                    ` : ''}
                    ${result.persistence_score ? `
                        <div class="score">
                            <span class="label">Persistence:</span>
                            <span class="value">${formatScore(result.persistence_score)}</span>
                        </div>
                    ` : ''}
                </div>
            `;
            
            searchResults.appendChild(resultCard);
        });
    }
    
    // Helper function to escape HTML
    function escapeHtml(unsafe) {
        return unsafe
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;")
            .replace(/"/g, "&quot;")
            .replace(/'/g, "&#039;");
    }
    
    // Check if there's a query in the URL (for shared links)
    const urlParams = new URLSearchParams(window.location.search);
    const urlQuery = urlParams.get('q');
    
    if (urlQuery) {
        queryInput.value = urlQuery;
        
        // Set scoring options from URL if present
        if (urlParams.has('quantum')) {
            quantumScoring.checked = urlParams.get('quantum') === '1';
        }
        if (urlParams.has('persistence')) {
            persistenceScoring.checked = urlParams.get('persistence') === '1';
        }
        
        // Automatically submit the search
        searchForm.dispatchEvent(new Event('submit'));
    }
    
    // Add animation effects for quantum-inspired visuals
    const searchBox = document.querySelector('.search-box');
    const logo = document.querySelector('.logo img');
    
    searchBox.addEventListener('focus', () => {
        document.body.classList.add('quantum-active');
    }, true);
    
    searchBox.addEventListener('blur', () => {
        document.body.classList.remove('quantum-active');
    }, true);
    
    // Animate logo on load
    logo.addEventListener('load', () => {
        logo.classList.add('resonating');
    });
});
# Rust Scraper for Path of Exile 2

## Problem Overview

### Modifier Weighting
The exact modifier weightings (i.e., the likelihood of a modifier appearing on items in Path of Exile 2) are not available in the game client and cannot be directly data-mined.

### Goal
The objective is to obtain an overview of modifier distributions (weights) by scraping data from the trade site. By matching item modifiers to actual game modifiers, you can infer the general distribution of those modifiers.

## Solution Approach

### Scraping Data
- The idea is to scrape item listings from the trade site to gather data on item modifiers.
- The data gathered will then be matched to actual game modifiers to get an understanding of their distribution across items.

### Normalization & Breakpoints
- The data will be normalized by applying breakpoints to the modifier weights, which will allow you to generate familiar distribution curves (such as a bell curve).
- The normalization script will also adjust data between base item groups to ensure consistency for similar modifiers across similar base types.

### Bias Consideration
- The trade site is biased towards higher-tier mods or more desirable modifiers (higher-value listings).
- The scraping should account for this bias by including a range of listings, particularly lower-value ones, as higher-value listings may skew the results due to their association with better mods.
- However, in some cases, the bias cannot be fully avoided, as certain rare and highly desirable modifiers may only appear significantly in higher-value listings. This bias is accounted for in part in the normalization process.

### Error Acknowledgement
- There will be some inherent errors or inconsistencies in the data due to the limitations of scraping and normalizing.
- Despite these potential errors, the goal is to achieve a rough estimate of which modifiers are rarer or more common in the game.

## Key Points of the Approach
- The solution involves scraping data, matching item modifiers, normalizing the data, and accounting for biases.
- The success of this solution depends on the balance of scraping items from both low and high-value listings to reduce the bias from higher-tier mods.
- The goal is to generate a broad overview of modifier distributions (and their weights) with an understanding of their relative rarity or commonness, despite potential inconsistencies in the data.

## Getting Started

### Prerequisites
- Rust programming language
- Access to the Path of Exile 2 trade site

### Installation
1. Clone the repository:
    ```sh
    git clone https://github.com/yourusername/rust-scraper.git
    ```
2. Navigate to the project directory:
    ```sh
    cd rust-scraper
    ```
3. Build the project:
    ```sh
    cargo build
    ```

### Usage
1. Run the scraper:
    ```sh
    cargo run
    ```
2. The scraped data will be saved in the `output` directory.

## Contributing
Contributions are welcome! Please open an issue or submit a pull request.

## License
This project is licensed under the MIT License.

## Acknowledgements
- Path of Exile 2 for providing the trade site.
- Rust community for their support and resources.

## Future Enhancements

### Additional Data Sources
- Incorporate data from other Path of Exile 2 community resources to enhance the accuracy of modifier weightings.
- Explore the possibility of using API endpoints provided by the game developers for more direct data access.

### Improved Normalization Techniques
- Develop more sophisticated normalization algorithms to better account for biases and inconsistencies in the scraped data.
- Implement machine learning models to predict modifier distributions based on historical data.

### User Interface
- Create a user-friendly interface to visualize the modifier distributions and allow users to interact with the data.
- Provide filtering options to let users focus on specific item types or modifiers.

### Automation
- Automate the scraping process to run at regular intervals, ensuring that the data remains up-to-date with the latest listings on the trade site.
- Set up notifications or alerts for significant changes in modifier distributions.
// status.js
async function updateStatusBar() {
    try {
        // Fetch data from the /procmon endpoint
        const response = await fetch('/procmon');

        if (!response.ok) {
            throw new Error('Failed to fetch data');
        }

        const data = await response.json();

        const cpuUsages = data.cpu_usage;
        const mid = Math.ceil(cpuUsages.length / 2);

        const firstHalf = cpuUsages
            .slice(0, mid)
            .map(usage => `${usage.toFixed(2)}%`)
            .join(' | ');

        const secondHalf = cpuUsages
            .slice(mid)
            .map(usage => `${usage.toFixed(2)}%`)
            .join(' | ');

        // Update the status bar fields
        document.querySelector('.status-bar-field:nth-child(1)').textContent = `CPU 0: ${cpuUsages[0].toFixed(2)}%`;
        document.querySelector('.status-bar-field:nth-child(2)').textContent =  `CPU 1: ${cpuUsages[1].toFixed(2)}%`;
        document.querySelector('.status-bar-field:nth-child(3)').textContent =  `CPU 2: ${cpuUsages[2].toFixed(2)}%`;
        document.querySelector('.status-bar-field:nth-child(4)').textContent =  `CPU 3: ${cpuUsages[3].toFixed(2)}%`;
        document.querySelector('.status-bar-field:nth-child(5)').textContent = `RAM Usage: ${data.ram_usage.toFixed(2)}%`;
        //document.querySelector('.status-bar-field:nth-child(5)').textContent = `Swap Usage: ${data.swap_usage.toFixed(2)}%`;
    } catch (error) {
        console.error('Error updating status bar:', error);
    }
}

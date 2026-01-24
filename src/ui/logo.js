async function loadLogo() {
    const response = await fetch('logo.svg');
    const svgText = await response.text();
    const parser = new DOMParser();
    const svgDoc = parser.parseFromString(svgText, 'image/svg+xml');
    const svg = svgDoc.documentElement;
    svg.classList.add('logo');
    
    // Add glow filter for laser effect
    const defs = document.createElementNS('http://www.w3.org/2000/svg', 'defs');
    defs.innerHTML = `
        <filter id="glow">
            <feGaussianBlur stdDeviation="3" result="coloredBlur"/>
            <feMerge>
                <feMergeNode in="coloredBlur"/>
                <feMergeNode in="SourceGraphic"/>
            </feMerge>
        </filter>
    `;
    svg.insertBefore(defs, svg.firstChild);
    
    document.body.appendChild(svg);
    
    // Animate paths progressively with laser point
    const paths = svg.querySelectorAll('path');
    paths.forEach((path, index) => {
        const length = path.getTotalLength();
        path.style.strokeDasharray = length;
        path.style.strokeDashoffset = length;
        
        // Create laser point
        const point = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
        point.setAttribute('r', '4');
        point.setAttribute('fill', '#00ff0000');
        point.setAttribute('filter', 'url(#glow)');
        point.classList.add('laser-point');

        svg.appendChild(point);
        
        const delay = index * 0.05;
        const duration = .5;
        
        // Animate the path drawing
        setTimeout(() => {
            path.style.transition = `stroke-dashoffset ${duration}s ease-out`;
            path.style.strokeDashoffset = '0';
            point.setAttribute('fill', '#00ffff');
            
            // Animate the laser point along the path
            let start = null;
            const animate = (timestamp) => {
                if (!start) start = timestamp;
                const progress = Math.min((timestamp - start) / (duration * 1000), 1);
                
                const currentLength = length * (1 - progress);
                const pointOnPath = path.getPointAtLength(length - currentLength);
                point.setAttribute('cx', pointOnPath.x);
                point.setAttribute('cy', pointOnPath.y);
                
                if (progress < 1) {
                    requestAnimationFrame(animate);
                } else {
                    point.style.opacity = '0';
                    setTimeout(() => point.remove(), 200);
                }
            };
            requestAnimationFrame(animate);
        }, delay * 1000);
    });
}

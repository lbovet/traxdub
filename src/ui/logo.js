async function loadLogo(ratio, bulletCallback, doneCallback) {
    const response = await fetch('logo.svg');
    const svgText = await response.text();
    const parser = new DOMParser();
    const svgDoc = parser.parseFromString(svgText, 'image/svg+xml');
    const svg = svgDoc.documentElement;
    svg.setAttribute('width', 206 * ratio);
    svg.classList.add('logo');
    svg.style.transform = 'scaleY(1)';

    // Center the logo SVG absolutely over the background
    svg.style.position = 'absolute';
    svg.style.left = '50%';
    svg.style.top = '25%';
    svg.style.transform = 'translate(-50%, -50%) scaleY(1)';
    svg.style.zIndex = '1';
    svg.style.pointerEvents = 'none';
    document.body.appendChild(svg);
    
    const duration = 1.2;
    const gap = 0.1; // seconds

    // Animate paths progressively with laser point
    const paths = svg.querySelectorAll('path');
    paths.forEach((path, index) => {
        const length = path.getTotalLength();
        path.style.strokeDasharray = length;
        path.style.strokeDashoffset = length;
        
        // Create laser point
        const point = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
        point.setAttribute('fill', '#00ff0000');
        point.setAttribute('filter', 'url(#blur)');
        point.classList.add('laser-point');

        svg.appendChild(point);
        
        const delay = index * gap;

        // Animate the path drawing
        setTimeout(() => {
            path.style.transition = `stroke-dashoffset ${duration}s ease-out`;
            path.style.strokeDashoffset = '0';
            path.style.stroke = '#1e2828'
            
            // Animate the laser point along the path
            let start = null;
            let highlight = null;
            let part = 0
            const animate = (timestamp) => {
                if (!start) start = timestamp;
                const progress = Math.min(2*(timestamp - start) / (duration * 1000), 2);
                
                const currentLength = Math.abs(length * (1 - progress));
                
                const pointOnPath = path.getPointAtLength(length - currentLength);
                point.setAttribute('cx', pointOnPath.x);
                point.setAttribute('cy', pointOnPath.y);
                point.setAttribute('fill', Math.random() > 0.2 ? '#66ffff' : '#ffffff');
                if (progress < 1) {                    
                    point.setAttribute('r', 2 + (2 + progress*10)*Math.random());
                    requestAnimationFrame(animate);
                } else if (progress < 2) {      
                    point.setAttribute('r', 14*Math.random());              
                    if (part === 0) {
                        // Start highlight animation
                        part = 1;                        
                        setTimeout(() => point.remove(), 400);
                        highlight = path.cloneNode();
                        highlight.setAttribute('id', 'highlight-'+index);
                        highlight.setAttribute('class', 'highlight-path');
                        highlight.style.strokeDashoffset = length+1;
                        highlight.style.stroke = '#66ffff' // lighter cyan
                        highlight.style.transition = `stroke-dashoffset ${duration*0.7}s ease-out`;
                        path.parentNode.insertBefore(highlight, path.nextSibling);
                        setTimeout(() => {
                            highlight.style.strokeDashoffset = 2*length;
                        }, 0);
                        requestAnimationFrame(animate);
                    } else {
                        // Fade out highlight
                        point.setAttribute('opacity', 2 - progress);
                        highlight.style.strokeWidth = 28 - progress * 12
                        const blur = svg.querySelector('filter#glow feGaussianBlur');                        
                        blur.setAttribute('stdDeviation', 20 - progress * 6);
                        highlight.style.strokeOpacity = 1 - Math.random() * (2 - progress);
                        const flood = svg.querySelector('filter#glow feFlood');                        
                        flood.setAttribute('flood-opacity', 1 - progress / 2);                
                        requestAnimationFrame(animate);
                    }
                }
            };
            requestAnimationFrame(animate);
        }, delay * 1000);
    });    

    //After all path animations, collapse the svg vertically
     const totalDelay = duration;
    setTimeout(() => {
        bulletCallback();
        setTimeout(() => {
            doneCallback();
            svg.style.transition = 'transform 0.7s, opacity 0.3s';
            svg.style.transform = 'translate(-50%, -50%) scaleY(0)';
            setTimeout(() => {
                svg.style.opacity = '0';
            }, 200);
            setTimeout(() => {
                svg.remove();
            }, 500);
        }, 200);         
    }, totalDelay * 1000 + 500);

}

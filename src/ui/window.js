let ratio = 1.0;
const size = 2000;
document.getElementById('main').setAttribute('width', size * ratio);
document.getElementById('main').setAttribute('height', size * ratio);

function init(callback) {
    window.addEventListener('load', async function() {
        await loadLogo(ratio, () => {
            const bullet = document.getElementById('bullet');
            bullet.style.transition = 'r 1.4s ease-out';
            bullet.setAttribute('r', 4);
        },
        () => {
            const line = document.getElementById('line');
            line.style.transition = 'stroke-opacity 0.4s ease-in';
            line.setAttribute('stroke-opacity', 1);
            const bullet = document.getElementById('bullet');
            bullet.style.transition = 'stroke-width 0.4s ease-in';
            bullet.setAttribute('stroke-width', 1);
            callback();
        });
        document.body.classList.add('loaded');
        window.ipc.postMessage('page-loaded');
    });
}

function updateSize() {
    const svg = document.getElementById('main');
    if (!svg) return;
    const pt = svg.createSVGPoint();
    pt.x = 0;
    pt.y = 0;
    const svgCoords = pt.matrixTransform(svg.getScreenCTM().inverse());

    // Move the graph vertically between the screen top and the center
    const graph = document.getElementById('graph');
    const newY = (size / 2 - Math.round(svgCoords.y)) * 0.5;
    graph.setAttribute('transform', `translate(0, ${-newY})`);
}
window.addEventListener('resize', updateSize);
window.addEventListener('DOMContentLoaded', updateSize);

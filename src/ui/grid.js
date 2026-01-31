function createGrid(svgElement) {
    let rowCount = 1;
    let columnCount = 2;
    let boxes = new Map(); // id -> { box, row, col, group }
    let lines = new Map(); // key -> { fromId, toId, path }
    let animatingBoxes = new Map(); // id -> { startPos, endPos, startTime, duration }
    let pendingChanges = new Set(); // Set of box ids with pending position changes
    let pendingSizeChange = false;

    const svgNS = "http://www.w3.org/2000/svg";
    const verticalSpacing = 48;
    const minHorizontalSpacing = 30;
    const maxHorizontalSpacing = 146;
    const boxMinWidth = 50;
    const boxHeight = 26;

    // Create container group for all boxes
    const containerGroup = document.createElementNS(svgNS, 'g');
    containerGroup.setAttribute('id', 'grid-container');
    svgElement.firstElementChild.firstElementChild.appendChild(containerGroup);

    // Create container for lines (behind boxes)
    const linesGroup = document.createElementNS(svgNS, 'g');
    linesGroup.setAttribute('id', 'lines-container');
    containerGroup.appendChild(linesGroup);

    // Create container for boxes (in front of lines)
    const boxesGroup = document.createElementNS(svgNS, 'g');
    boxesGroup.setAttribute('id', 'boxes-container');
    containerGroup.appendChild(boxesGroup);

    function getHorizontalSpacing() {
        if (columnCount <= 1) return maxHorizontalSpacing;
        // Interpolate between max and min based on column count
        // At 2 columns: maxHorizontalSpacing, approaches minHorizontalSpacing as columns increase
        const ratio = Math.max(0, (columnCount - 2) / 10); // Reaches min at ~12 columns
        return maxHorizontalSpacing - ratio * (maxHorizontalSpacing - minHorizontalSpacing);
    }

    function getCellPosition(row, col) {
        const hSpacing = getHorizontalSpacing();
        const gridWidth = (columnCount - 1) * hSpacing;

        // Use rowCount - 1 for even row counts to avoid position shifts
        const effectiveRowCount = rowCount % 2 === 0 ? rowCount - 1 : rowCount;
        const gridHeight = (effectiveRowCount - 1) * verticalSpacing;

        // Get SVG viewBox to center the grid
        const viewBox = svgElement.getAttribute('viewBox').split(' ');
        const svgWidth = parseFloat(viewBox[2]);
        const svgHeight = parseFloat(viewBox[3]);

        const centerX = svgWidth / 2;
        const centerY = svgHeight / 2;

        const startX = centerX - gridWidth/2 - boxMinWidth/2;;
        const startY = centerY - gridHeight / 2;

        return {
            x: startX + col * hSpacing,
            y: startY + row * verticalSpacing
        };
    }

    function setSize(newRowCount, newColumnCount) {
        rowCount = newRowCount;
        columnCount = newColumnCount;
        pendingSizeChange = true;

        // Mark all boxes as having pending changes
        boxes.forEach((_, id) => {
            pendingChanges.add(id);
        });
    }

    function setBox(id, box, row, col) {
        const pos = getCellPosition(row, col);

        if (boxes.has(id)) {
            // Update existing box
            const existing = boxes.get(id);
            existing.box = box;
            existing.row = row;
            existing.col = col;

            const text = existing.group.querySelector('text');
            const rect = existing.group.querySelector('rect');
            text.textContent = box.label;

            // Calculate text width and update box width
            const textBBox = text.getBBox();
            const boxWidth = Math.max(boxMinWidth, textBBox.width + 10); // 10 units padding
            rect.setAttribute('width', boxWidth);
            rect.setAttribute('x', '0');
            text.setAttribute('x', boxWidth / 2); // Center text in box

            // Handle invisible boxes
            if (box.invisible) {
                rect.style.opacity = '0';
                text.style.opacity = '0';
            } else {
                rect.style.opacity = '1';
                text.style.opacity = '1';
            }

            // Mark this box as having pending changes
            pendingChanges.add(id);
        } else {
            // Create new box
            const group = document.createElementNS(svgNS, 'g');
            group.setAttribute('transform', `translate(${pos.x}, ${pos.y})`);

            const text = document.createElementNS(svgNS, 'text');
            text.setAttribute('x', '0'); // Will be updated after measuring
            text.setAttribute('y', '1.5');
            text.setAttribute('text-anchor', 'middle');
            text.setAttribute('dominant-baseline', 'middle');
            text.setAttribute('fill', '#66ffff');
            text.setAttribute('font-size', '20');
            text.setAttribute('opacity', '0'); // Start invisible for animation
            text.textContent = box.label;

            // Add text first to measure it
            group.appendChild(text);
            boxesGroup.appendChild(group);

            // Measure text to calculate box width
            const textBBox = text.getBBox();
            const boxWidth = Math.max(boxMinWidth, textBBox.width + 10); // 10 units padding

            // Center text in box
            text.setAttribute('x', boxWidth / 2);

            const rect = document.createElementNS(svgNS, 'rect');
            rect.setAttribute('x', '0');
            rect.setAttribute('y', '0'); // Start at center
            rect.setAttribute('width', boxWidth);
            rect.setAttribute('height', 0); // Start collapsed
            rect.setAttribute('stroke', '#66ffff');
            rect.setAttribute('stroke-width', '1');
            rect.setAttribute('fill', '#1a1a1a');
            rect.setAttribute('opacity', box.invisible ? '0' : '1'); // Start invisible for animation
            rect.setAttribute('rx', '0.3');

            // Insert rect before text to render behind it
            group.insertBefore(rect, text);

            boxes.set(id, { box, row, col, group });

            // Animate expansion
            setTimeout(() => {
                rect.style.transition = 'height 200ms ease-in-out, y 200ms ease-in-out';
                text.style.transition = 'opacity 200ms ease-in-out';
                rect.setAttribute('height', boxHeight);
                rect.setAttribute('y', -boxHeight / 2);

                // Only show if not invisible
                if (!box.invisible) {
                    text.setAttribute('opacity', '1');
                }
            }, 20);
        }
    }

    function removeBox(id) {
        if (!boxes.has(id)) return;

        const { group } = boxes.get(id);
        const rect = group.querySelector('rect');
        const text = group.querySelector('text');

        // Animate collapse
        rect.style.transition = 'height 250ms ease-in-out, y 250ms ease-in-out';
        text.style.transition = 'opacity 250ms ease-in-out';
        rect.setAttribute('height', '0');
        rect.setAttribute('y', '0');
        text.setAttribute('opacity', '0');

        setTimeout(() => {
            boxesGroup.removeChild(group);
            boxes.delete(id);
        }, 250);
    }

    function getBoxEnds(id) {
        if (!boxes.has(id)) return null;

        const { box, group, row, col } = boxes.get(id);
        const rect = group.querySelector('rect');
        const width = parseFloat(rect.getAttribute('width'));

        let x, y;

        // Check if box is currently animating
        if (animatingBoxes.has(id)) {
            const { startPos, endPos, startTime, duration } = animatingBoxes.get(id);
            const elapsed = Date.now() - startTime;
            const progress = Math.min(elapsed / duration, 1);

            // Linear interpolation
            x = startPos.x + (endPos.x - startPos.x) * progress;
            y = startPos.y + (endPos.y - startPos.y) * progress;
        } else {
            const pos = getCellPosition(row, col);
            x = pos.x;
            y = pos.y;
        }

        // Swap connection points for invisible boxes
        if (box.invisible) {
            return {
                inX: x + width,   // Right edge for invisible - incoming
                outX: x,          // Left edge for invisible - outgoing
                y: y
            };
        } else {
            return {
                inX: x,           // Left edge - incoming line attachment point
                outX: x + width,  // Right edge - outgoing line attachment point
                y: y
            };
        }
    }

    function calculateLinePath(fromId, toId) {
        const fromBox = getBoxEnds(fromId);
        const toBox = getBoxEnds(toId);

        if (!fromBox || !toBox) return '';

        // First segment: horizontal from right edge of fromBox, 15 units long
        const x1 = fromBox.outX;
        const y1 = Math.floor(fromBox.y);
        const x2 = x1 + 15;
        const y2 = Math.floor(y1);

        // Third segment: horizontal to left edge of toBox, starting 15 units before
        const x4 = toBox.inX;
        const y4 = Math.floor(toBox.y);
        const x3 = x4 - 15;
        const y3 = Math.floor(y4);

        // If boxes are aligned horizontally, make middle segment straight
        if (y1 === y4) {
            return `M ${x1} ${y1} L ${x4} ${y4}`;
        } else {
           // Create path with three segments
            return `M ${x1} ${y1} L ${x2} ${y2} L ${x3} ${y3} L ${x4} ${y4}`;
        }
    }

    function updateAllLines() {
        lines.forEach(({ fromId, toId, path }) => {
            const pathData = calculateLinePath(fromId, toId);
            path.setAttribute('d', pathData);
        });
    }

    function addLine(fromId, toId) {
        const key = `${fromId}-${toId}`;

        // Remove existing line if any
        if (lines.has(key)) {
            const { path } = lines.get(key);
            linesGroup.removeChild(path);
        }

        // Create new line
        const path = document.createElementNS(svgNS, 'path');
        path.setAttribute('stroke', '#66ffff');
        // make round line joins
        path.setAttribute('stroke-linejoin', 'round');
        path.setAttribute('stroke-width', '1');
        path.setAttribute('fill', 'none');

        const pathData = calculateLinePath(fromId, toId);
        path.setAttribute('d', pathData);

        linesGroup.appendChild(path);
        lines.set(key, { fromId, toId, path });
    }

    function removeLine(fromId, toId) {
        const key = `${fromId}-${toId}`;

        if (!lines.has(key)) return;

        const { path } = lines.get(key);
        linesGroup.removeChild(path);
        lines.delete(key);
    }

    function commit() {
        if (pendingChanges.size === 0) return;

        const startTime = Date.now();
        const duration = 300;

        // Animate all boxes with pending changes
        pendingChanges.forEach(id => {
            if (!boxes.has(id)) return;

            const { row, col, group } = boxes.get(id);
            const currentTransform = group.getAttribute('transform');
            const match = currentTransform.match(/translate\(([^,]+),\s*([^)]+)\)/);
            const startPos = match ? { x: parseFloat(match[1]), y: parseFloat(match[2]) } : { x: 0, y: 0 };
            const endPos = getCellPosition(row, col);

            animatingBoxes.set(id, { startPos, endPos, startTime, duration });

            group.style.transition = 'transform 300ms linear';
            group.setAttribute('transform', `translate(${endPos.x}, ${endPos.y})`);
        });

        // Clear pending changes
        pendingChanges.clear();
        pendingSizeChange = false;

        // Continuously update lines during animation
        function animateLines() {
            const elapsed = Date.now() - startTime;
            updateAllLines();

            if (elapsed < duration) {
                requestAnimationFrame(animateLines);
            } else {
                // Clear animation state
                animatingBoxes.clear();
            }
        }

        requestAnimationFrame(animateLines);
    }

    return {
        setSize,
        setBox,
        removeBox,
        addLine,
        removeLine,
        commit
    };
}

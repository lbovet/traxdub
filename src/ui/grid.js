function createGrid(svgElement) {
    let rowCount = 1;
    let columnCount = 2;
    let boxes = new Map(); // id -> { box, row, col, group }
    let lines = new Map(); // key -> { fromId, toId, path }
    let animatingBoxes = new Map(); // id -> { startPos, endPos, startTime, duration }
    let pendingChanges = new Set(); // Set of box ids with pending position changes
    let pendingSizeChange = false;
    let firstCommit = true;
    let focusCircle = null; // Circle to indicate focused line
    let focusedElement = null; // { type: 'box'|'line', id: string }

    const svgNS = "http://www.w3.org/2000/svg";
    const verticalSpacing = 48;
    const minHorizontalSpacing = 30;
    const maxHorizontalSpacing = 146;
    const boxMinWidth = 50;
    const boxHeight = 26;

    // Color functions
    function getBoxHighlightColor(box) {
        if(box.active === false) {
            return '#067575';
        } else {
            return '#66ffff';
        }
    }

    function getLineHighlightColor(fromBox, toBox) {
        if((fromBox.active === false) ||  (toBox.active === false)) {
            return '#067575';
        } else {
            return '#66ffff';
        }
    }

    function getBackgroundColor(element) {
        return '#1a1a1a';
    }

    function updateBoxStates() {
        if (!focusedElement) {
            // No focus - mark all boxes as active
            boxes.forEach(({ box }) => {
                box.active = true;
            });
            return;
        }

        // Determine starting box(es) based on focused element
        let startBoxIds = [];
        if (focusedElement.type === 'box') {
            startBoxIds = [focusedElement.id];
        } else if (focusedElement.type === 'line') {
            const [fromId, toId] = focusedElement.id.split('-');
            startBoxIds = [fromId, toId];
        }

        // Build adjacency lists for upstream and downstream navigation
        const downstream = new Map(); // boxId -> [connected boxIds]
        const upstream = new Map();   // boxId -> [connected boxIds]

        boxes.forEach((_, id) => {
            downstream.set(id, []);
            upstream.set(id, []);
        });

        lines.forEach(({ fromId, toId }) => {
            downstream.get(fromId)?.push(toId);
            upstream.get(toId)?.push(fromId);
        });

        // BFS strictly downstream from starting boxes
        const reachable = new Set(startBoxIds);
        let downstreamQueue = [...startBoxIds];
        const downstreamVisited = new Set(startBoxIds);

        while (downstreamQueue.length > 0) {
            const currentId = downstreamQueue.shift();
            console.log(`Visiting downstream: ${currentId}`);

            const downstreamBoxes = downstream.get(currentId) || [];
            for (const nextId of downstreamBoxes) {
                if (!downstreamVisited.has(nextId)) {
                    console.log(`  Found downstream: ${nextId}`);
                    downstreamVisited.add(nextId);
                    reachable.add(nextId);
                    downstreamQueue.push(nextId);
                }
            }
        }

        // BFS strictly upstream from starting boxes
        let upstreamQueue = [...startBoxIds];
        const upstreamVisited = new Set(startBoxIds);

        while (upstreamQueue.length > 0) {
            const currentId = upstreamQueue.shift();
            console.log(`Visiting upstream: ${currentId}`);

            const upstreamBoxes = upstream.get(currentId) || [];
            for (const nextId of upstreamBoxes) {
                if (!upstreamVisited.has(nextId)) {
                    console.log(`  Found upstream: ${nextId}`);
                    upstreamVisited.add(nextId);
                    reachable.add(nextId);
                    upstreamQueue.push(nextId);
                }
            }
        }

        // Update active property for all boxes
        boxes.forEach(({ box }, id) => {
            console.log(`Box ${id} active: ${reachable.has(id)}`);
            box.active = reachable.has(id);
        });
    }

    function updateElementColors() {
        // Update all box colors
        boxes.forEach(({ box, group }, id) => {
            const rect = group.querySelector('rect');
            const text = group.querySelector('text');
            const color = getBoxHighlightColor(box);

            rect.setAttribute('stroke', color);

            // Check if this box is focused
            if (focusedElement && focusedElement.type === 'box' && focusedElement.id === id) {
                // Keep inverted colors for focused box
                rect.setAttribute('fill', color);
                text.setAttribute('fill', getBackgroundColor());
            } else {
                rect.setAttribute('fill', getBackgroundColor());
                text.setAttribute('fill', color);
            }
        });

        // Update all line colors
        lines.forEach(({ fromId, toId, path }, key) => {
            const fromBox = boxes.get(fromId)?.box;
            const toBox = boxes.get(toId)?.box;
            path.setAttribute('stroke', getLineHighlightColor(fromBox, toBox));
        });

        // Update focus circle color if it exists and a line is focused
        if (focusCircle && focusedElement && focusedElement.type === 'line') {
            const [fromId, toId] = focusedElement.id.split('-');
            focusCircle.setAttribute('fill', getLineHighlightColor(
                boxes.get(fromId)?.box,
                boxes.get(toId)?.box
            ));
        }
    }

    // Create container group for all boxes
    const containerGroup = document.createElementNS(svgNS, 'g');
    containerGroup.setAttribute('id', 'grid-container');
    containerGroup.style.opacity = '0'; // Start invisible
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
            if(box.label) {
                text.textContent = box.label;
            }

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
            text.setAttribute('fill', getBoxHighlightColor(box));
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
            rect.setAttribute('stroke', getBoxHighlightColor(box));
            rect.setAttribute('stroke-width', '1');
            rect.setAttribute('fill', getBackgroundColor());
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
        boxes.delete(id);

        setTimeout(() => {
            group.parentNode.removeChild(group);
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
        const fromBox = boxes.get(fromId)?.box;
        const toBox = boxes.get(toId)?.box;
        const path = document.createElementNS(svgNS, 'path');
        path.setAttribute('stroke', getLineHighlightColor(fromBox, toBox));
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
        path.parentNode.removeChild(path);
        lines.delete(key);
    }

    function unfocus() {
        if (!focusedElement) return;

        if (focusedElement.type === 'line') {
            // Hide the focus circle
            if (focusCircle) {
                focusCircle.style.display = 'none';
            }
        } else if (focusedElement.type === 'box') {
            // Uninvert the box
            const { box, group } = boxes.get(focusedElement.id);
            if (group) {
                const rect = group.querySelector('rect');
                const text = group.querySelector('text');
                rect.setAttribute('fill', getBackgroundColor(box));
                text.setAttribute('fill', getBoxHighlightColor(box));
                text.style.fontWeight = 'normal';
            }
        }

        focusedElement = null;
    }

    function focusLine(fromId, toId) {
        const key = `${fromId}-${toId}`;

        if (!lines.has(key)) return;

        // Unfocus any currently focused element
        unfocus();

        const { path } = lines.get(key);

        // Get the total length of the path
        const pathLength = path.getTotalLength();

        // Get the point at the middle of the path
        const midPoint = path.getPointAtLength(pathLength / 2);

        // Create focus circle if it doesn't exist
        if (!focusCircle) {
            focusCircle = document.createElementNS(svgNS, 'circle');
            focusCircle.setAttribute('r', '4');
            focusCircle.setAttribute('fill', getLineHighlightColor(
                boxes.get(fromId)?.box,
                boxes.get(toId)?.box
            ));
            focusCircle.setAttribute('stroke', getBackgroundColor());
            focusCircle.setAttribute('stroke-width', '1');
            linesGroup.appendChild(focusCircle);
        }

        // Position the circle at the midpoint
        focusCircle.setAttribute('cx', midPoint.x);
        focusCircle.setAttribute('cy', midPoint.y);
        focusCircle.style.display = 'block';

        // Track focused element
        focusedElement = { type: 'line', id: key };
    }

    function focusBox(id) {
        if (!boxes.has(id)) return;

        // Unfocus any currently focused element
        unfocus();

        const { box, group } = boxes.get(id);
        const text = group.querySelector('text');

        // Set font weight
        text.style.fontWeight = 'bold';

        // Track focused element
        focusedElement = { type: 'box', id };
    }

    function commit() {
        // Fade in grid on first commit
        if (firstCommit) {
            containerGroup.style.transition = 'opacity 300ms ease-in';
            containerGroup.style.opacity = '1';
            firstCommit = false;
        }

        // Update box states and colors
        updateBoxStates();
        updateElementColors();

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
        focusLine,
        focusBox,
        commit
    };
}

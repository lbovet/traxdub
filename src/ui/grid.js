function createGrid(svgElement) {
    let rowCount = 1;
    let columnCount = 2;
    let boxes = new Map(); // id -> { box, row, col, group }
    let lines = new Map(); // key -> { fromId, toId, path }

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

        // Animate all existing boxes to their new positions
        boxes.forEach(({ box, row, col, group }) => {
            const pos = getCellPosition(row, col);
            const rect = group.querySelector('rect');
            const text = group.querySelector('text');

            group.style.transition = 'transform 300ms ease-in-out';
            group.setAttribute('transform', `translate(${pos.x}, ${pos.y})`);
        });

        // Update all lines
        updateAllLines();
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

            // Animate to new position
            existing.group.style.transition = 'transform 300ms ease-in-out';
            existing.group.setAttribute('transform', `translate(${pos.x}, ${pos.y})`);

            // Update lines connected to this box
            updateLinesForBox(id);
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
            text.setAttribute('opacity', '0'); // Start invisible
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
                text.setAttribute('opacity', '1');
            }, 20);
        }
    }

    function removeBox(id) {
        if (!boxes.has(id)) return;

        const { group } = boxes.get(id);
        const rect = group.querySelector('rect');
        const text = group.querySelector('text');

        // Animate collapse
        rect.style.transition = 'height 300ms ease-in-out, y 300ms ease-in-out';
        text.style.transition = 'opacity 300ms ease-in-out';
        rect.setAttribute('height', '0');
        rect.setAttribute('y', '0');
        text.setAttribute('opacity', '0');

        setTimeout(() => {
            boxesGroup.removeChild(group);
            boxes.delete(id);
        }, 300);
    }

    function getBoxDimensions(id) {
        if (!boxes.has(id)) return null;

        const { group, row, col } = boxes.get(id);
        const rect = group.querySelector('rect');
        const pos = getCellPosition(row, col);
        const width = parseFloat(rect.getAttribute('width'));

        return {
            x: pos.x,
            y: pos.y,
            width: width,
            height: boxHeight
        };
    }

    function calculateLinePath(fromId, toId) {
        const fromBox = getBoxDimensions(fromId);
        const toBox = getBoxDimensions(toId);

        if (!fromBox || !toBox) return '';

        // First segment: horizontal from right middle of fromBox, 10 units long
        const x1 = fromBox.x + fromBox.width;
        const y1 = Math.floor(fromBox.y);
        const x2 = x1 + 15;
        const y2 = Math.floor(y1);

        // Third segment: horizontal to left middle of toBox, starting 10 units before
        const x4 = toBox.x;
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

    function updateLinesForBox(boxId) {
        lines.forEach(({ fromId, toId, path }, key) => {
            if (fromId === boxId || toId === boxId) {
                const pathData = calculateLinePath(fromId, toId);
                path.setAttribute('d', pathData);
            }
        });
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

    return {
        setSize,
        setBox,
        removeBox,
        addLine
    };
}

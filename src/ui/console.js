// Console forwarding to Rust debug logging
// This must be loaded first to catch all JavaScript errors
// Only active when running in wry WebView (window.ipc exists)

(function() {
    // Check if we're running in the wry WebView
    if (typeof window.ipc === 'undefined') {        
        console.log('Running in normal browser - console forwarding disabled');
        return;
    }
    
    const originalLog = console.log;
    const originalWarn = console.warn;
    const originalError = console.error;
    
    // Extract file and line info from stack trace
    function getCallerInfo() {
        try {
            const stack = new Error().stack;
            // Stack trace format varies, but typically has lines like:
            // "at functionName (file.js:line:col)" or "at file.js:line:col"
            const lines = stack.split('\n');
            // Skip first 3 lines: Error, getCallerInfo, and the console override
            const callerLine = lines[3] || lines[2] || '';
            
            // Try to extract file:line info
            const match = callerLine.match(/([^\/\s]+\.js):(\d+):(\d+)/);
            if (match) {
                return `${match[1]}:${match[2]}`;
            }
            
            // Fallback: try to extract just the filename
            const fileMatch = callerLine.match(/([^\/\s]+\.js)/);
            if (fileMatch) {
                return fileMatch[1];
            }
            
            return '';
        } catch (e) {
            return '';
        }
    }
    
    // Get full stack trace
    function getStackTrace() {
        try {
            const stack = new Error().stack;
            const lines = stack.split('\n');
            // Skip first 3 lines and return the rest
            return lines.slice(3).join('\n');
        } catch (e) {
            return '';
        }
    }
    
    console.log = function(...args) {
        const caller = getCallerInfo();
        const message = args.map(arg => 
            typeof arg === 'object' ? JSON.stringify(arg) : String(arg)
        ).join(' ');
        const fullMessage = caller ? `[${caller}] ${message}` : message;
        window.ipc.postMessage(JSON.stringify({
            type: 'console_log',
            data: { message: fullMessage }
        }));
        originalLog.apply(console, args);
    };
    
    console.warn = function(...args) {
        const caller = getCallerInfo();
        const stack = getStackTrace();
        const message = args.map(arg => 
            typeof arg === 'object' ? JSON.stringify(arg) : String(arg)
        ).join(' ');
        const fullMessage = caller ? `[${caller}] ${message}` : message;
        const messageWithStack = stack ? `${fullMessage}\n${stack}` : fullMessage;
        window.ipc.postMessage(JSON.stringify({
            type: 'console_warn',
            data: { message: messageWithStack }
        }));
        originalWarn.apply(console, args);
    };
    
    console.error = function(...args) {
        const caller = getCallerInfo();
        const stack = getStackTrace();
        const message = args.map(arg => 
            typeof arg === 'object' ? JSON.stringify(arg) : String(arg)
        ).join(' ');
        const fullMessage = caller ? `[${caller}] ${message}` : message;
        const messageWithStack = stack ? `${fullMessage}\n${stack}` : fullMessage;
        window.ipc.postMessage(JSON.stringify({
            type: 'console_error',
            data: { message: messageWithStack }
        }));
        originalError.apply(console, args);
    };
    
    // Catch unhandled errors
    window.addEventListener('error', (event) => {
        const location = event.filename ? `${event.filename}:${event.lineno}:${event.colno}` : 'unknown';
        const message = `Uncaught error at ${location}: ${event.message}`;
        window.ipc.postMessage(JSON.stringify({
            type: 'console_error',
            data: { message }
        }));
    });
    
    // Catch unhandled promise rejections
    window.addEventListener('unhandledrejection', (event) => {
        const message = `Unhandled promise rejection: ${event.reason}`;
        window.ipc.postMessage(JSON.stringify({
            type: 'console_error',
            data: { message }
        }));
    });

    console.log('Console forwarding initialized');
})();



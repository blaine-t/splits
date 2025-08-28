// Application state
let startTime = 0;
let timerInterval = null;
let isRunning = false;
let currentScreen = 0;
let duration = 0;
let userData = {
    username: '',
    isDown: null,
    isElevator: null,
    carryingItems: null
};

// Screen management
const screens = [
    'start-screen',
    'timer-screen', 
    'username-screen',
    'direction-screen',
    'method-screen',
    'carrying-screen',
    'loading-screen'
];

// Initialize app
document.addEventListener('DOMContentLoaded', function () {
    // Load username from localStorage
    const savedUsername = localStorage.getItem('username');
    if (savedUsername) {
        userData.username = savedUsername;
        document.getElementById('username-input').value = savedUsername;
    }
    
    // Show first screen
    showScreen(0);
});

function showScreen(screenIndex) {
    // Hide all screens
    screens.forEach((screenId, index) => {
        const screen = document.getElementById(screenId);
        if (screen) {
            screen.classList.remove('active', 'prev');
            if (index < screenIndex) {
                screen.classList.add('prev');
            }
        }
    });
    
    // Show current screen
    const currentScreenElement = document.getElementById(screens[screenIndex]);
    if (currentScreenElement) {
        setTimeout(() => {
            currentScreenElement.classList.add('active');
        }, 50);
    }
    
    currentScreen = screenIndex;
}

function nextScreen() {
    if (currentScreen < screens.length - 1) {
        showScreen(currentScreen + 1);
    }
}

// Timer functions
function startTimer() {
    startTime = Date.now();
    isRunning = true;
    timerInterval = setInterval(updateTimer, 16); // ~60fps
    nextScreen();
}

function stopTimer() {
    if (!isRunning) return;
    
    const endTime = Date.now();
    duration = endTime - startTime;
    
    clearInterval(timerInterval);
    isRunning = false;
    
    nextScreen();
}

function updateTimer() {
    const elapsed = Date.now() - startTime;
    const minutes = Math.floor(elapsed / 60000);
    const seconds = Math.floor((elapsed % 60000) / 1000);
    const milliseconds = elapsed % 1000;
    
    document.getElementById('timer-display').textContent =
        `${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}:${milliseconds.toString().padStart(3, '0')}`;
}

// Form handlers
function confirmUsername() {
    const username = document.getElementById('username-input').value.trim();
    if (!username) {
        alert('Please enter a username');
        return;
    }
    
    userData.username = username;
    localStorage.setItem('username', username);
    nextScreen();
}

function selectDirection(direction) {
    userData.isDown = direction === 'down';
    nextScreen();
}

function selectMethod(method) {
    userData.isElevator = method === 'elevator';
    
    if (method === 'stairs') {
        nextScreen(); // Go to carrying screen
    } else {
        userData.carryingItems = null;
        submitSplit(); // Skip carrying screen for elevator
    }
}

function selectCarrying(carrying) {
    userData.carryingItems = carrying;
    submitSplit();
}

async function submitSplit() {
    showScreen(6); // Show loading screen
    
    const splitData = {
        user: userData.username,
        is_down: userData.isDown,
        is_elevator: userData.isElevator,
        duration_ms: duration
    };
    
    // Add carrying items to the data if applicable
    if (userData.carryingItems !== null) {
        splitData.carrying_items = userData.carryingItems;
    }
    
    try {
        const response = await fetch('api/v0/split/new', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify(splitData)
        });
        
        if (response.ok) {
            showSuccessMessage();
        } else {
            showErrorMessage('Failed to record split');
        }
    } catch (error) {
        console.error('Error:', error);
        showErrorMessage('Error recording split');
    }
}

function showSuccessMessage() {
    document.getElementById('loading-content').innerHTML = `
        <h2>Success!</h2>
        <p class="success-message">Split recorded successfully!</p>
        <button class="form-button" onclick="resetApp()" style="margin-top: 20px;">Start New Split</button>
    `;
}

function showErrorMessage(message) {
    document.getElementById('loading-content').innerHTML = `
        <h2>Error</h2>
        <p class="success-message">${message}</p>
        <button class="form-button" onclick="resetApp()" style="margin-top: 20px;">Try Again</button>
    `;
}

function resetApp() {
    // Reset all state
    startTime = 0;
    duration = 0;
    isRunning = false;
    userData.isDown = null;
    userData.isElevator = null;
    userData.carryingItems = null;
    
    if (timerInterval) {
        clearInterval(timerInterval);
        timerInterval = null;
    }
    
    // Reset timer display
    document.getElementById('timer-display').textContent = '00:00:000';
    
    // Reset loading content
    document.getElementById('loading-content').innerHTML = `
        <div class="loading-spinner"></div>
        <h2>Recording Split...</h2>
    `;
    
    // Go back to start screen
    showScreen(0);
}

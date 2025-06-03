const { exec } = require('child_process');

let tileServerProcess = null;

// Function to start TileServer
function startTileServer(port) {
  tileServerProcess = exec(`node ./node_modules/tileserver-gl -c ./src/map/mapConfig.json -p ${port}`, (error, stdout, stderr) => {
    if (error) {
      console.error(`Error starting TileServer: ${error}`);
      return;
    }
    console.log(`TileServer stdout: ${stdout}`);
    console.error(`TileServer stderr: ${stderr}`);
  });
  console.log(`TileServer started on port ${port}`);
}

// Function to stop TileServer with timeout
function stopTileServer(port) {
  return new Promise((resolve, reject) => {
    if (tileServerProcess) {
      console.log('Stopping TileServer...');

      let terminated = false;
      //freePort(port);
      // Set a timer for forcing the process to terminate
      const killTimeout = setTimeout(() => {
        console.warn('TileServer process did not stop in time, forcing kill...');
        tileServerProcess.kill('SIGKILL'); // Forcefully kill the process
      }, 1000); // 1 seconds to stop gracefully

      // Attempt graceful shutdown
      tileServerProcess.on('close', (code, signal) => {
        clearTimeout(killTimeout); // Clear the forced kill timeout
        console.log('TileServer stopped.');
        terminated = true;
        resolve();
      });

      tileServerProcess.kill('SIGTERM'); // Request termination

      // Check after timeout if the process did not stop
      setTimeout(() => {
        if (!terminated) {
          console.error('TileServer did not terminate in time.');
          resolve();  // Force resolve here, even if termination was not graceful
        }
      }, 3000); // Allow 3 seconds in total
    } else {
      console.log('TileServer is not running.');
      resolve();
    }
  });
}

function freePort(port) {
  exec(`netstat -ano | findstr :${port}`, (err, stdout) => {
    if (err) {
      console.error(`Error checking port ${port}:`, err);
      return;
    }

    const lines = stdout.trim().split('\n');
    if (lines.length === 0 || lines[0] === '') {
      console.log(`No process is using port ${port}.`);
      return;
    }

    lines.forEach((line) => {
      const parts = line.trim().split(/\s+/);
      const pid = parts[parts.length - 1]; // Extract the PID

      if (pid) {
        console.log(`Killing process with PID: ${pid} on port ${port}`);
        exec(`taskkill /PID ${pid} /F`, (killErr) => {
          if (killErr) {
            console.error(`Failed to kill PID ${pid}:`, killErr);
          } else {
            console.log(`Successfully killed process ${pid} using port ${port}.`);
          }
        });
      }
    });
  });
}

// Function to restart TileServer
async function restartTileServer(port) {
  try {
    await stopTileServer(port);

    // Add a slight delay to ensure the OS releases the port
    await new Promise(resolve => setTimeout(resolve, 500)); // 500ms delay

    startTileServer(port);
  } catch (error) {
    console.error('Error restarting TileServer:', error);
  }
}

module.exports = {
  startTileServer,
  stopTileServer,
  restartTileServer,
  tileServerProcess
};


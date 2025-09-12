# Z-Axis Device Positioning

The IoTCraft desktop client now supports full 3D positioning of lamp devices, allowing you to move them along all three axes (X, Y, and Z) using a professional 3D modeling approach.

## How It Works

### Drag Modes

The system uses three different drag modes that constrain movement to specific planes:

- **XZ Mode** (default): Move devices on the horizontal ground plane (X and Z axes), with Y locked
- **XY Mode**: Move devices on a vertical plane facing you (X and Y axes), with Z locked  
- **YZ Mode**: Move devices on a vertical side plane (Y and Z axes), with X locked

### Controls

#### Basic Dragging
- **Right-click and drag** any lamp device to move it in the current drag mode
- **Left-click** a lamp to toggle it on/off (unchanged)
- **ESC** to cancel dragging

#### Switching Drag Modes
- **X key**: Switch to YZ plane mode (locks X-axis, move in Y/Z)
- **Y key**: Switch to XZ plane mode (locks Y-axis, move in X/Z) - default ground movement
- **Z key**: Switch to XY plane mode (locks Z-axis, move in X/Y)

### Visual Feedback

When dragging a device, you'll see:

- **Yellow outline**: Indicates the device is being dragged
- **Colored axis lines**: 
  - Red line = X-axis
  - Green line = Y-axis  
  - Blue line = Z-axis
- **Bright vs. Dim axes**: Active axes (can move) are bright, locked axis is dim
- **Colored spheres**: Axis indicators at the end of each axis line

## Usage Examples

### Moving a Buried Lamp Up from Terrain

1. Right-click and drag the lamp horizontally to position it roughly where you want it in X/Z
2. Press **X key** to switch to YZ mode (this locks the X position)
3. Right-click and drag the lamp upward to lift it out of the terrain
4. Press **Y key** to return to ground plane mode for further X/Z adjustments

### Positioning a Lamp on a Wall

1. Press **Z key** to switch to XY mode
2. Right-click and drag to position the lamp on the wall surface
3. Press **Y key** to switch to XZ mode if you need to adjust depth

### Fine-Tuning Position

Use the keyboard shortcuts to quickly switch between planes and make precise adjustments:
- Get the horizontal position right in XZ mode
- Switch to XY or YZ mode to adjust height and depth
- Switch back to fine-tune as needed

## Technical Details

- Position updates are sent via MQTT to physical devices when you release the drag
- The system uses ray-plane intersection for accurate 3D positioning
- All three coordinates (X, Y, Z) are preserved and transmitted to devices
- Visual gizmos help you understand which axes are active

## Troubleshooting

**Lamp won't move in expected direction:**
- Check which drag mode you're in by looking at the axis gizmo colors
- Press the appropriate key (X, Y, or Z) to switch to the correct plane

**Can't lift lamp from terrain:**
- Make sure you're in XY or YZ mode (not the default XZ ground mode)
- Press X or Z key to enable vertical movement

**Lost track of which mode you're in:**
- Look at the axis gizmos: bright lines show movement axes, dim lines show locked axes
- Check the console logs for mode switch confirmations

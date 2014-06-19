// Jack Cruden
// MDDN242: Project 3
// Victoria University of Wellington

// At times the code in this program may be difficult to understand.
// I have tried to comment as much as possible, for my own benefit as
// well as for anyone browsing.

// Class structure:
// Game
// > Level
// -> Pixels
// -> Pipes
// > Player
// > Shots

// Setup variables
int canvasX = 1035; // (1035)
int canvasY = 780; // (780)
int size = 15; // (15) Size of pixels
int numX = canvasX/size; // Number of Pixels horizontally
int numY = canvasY/size; // Number of Pixels vertically
int penSize = 4; // How many pixels wide is the pen

// States
final int gas = 0;
final int liquid = 1;
final int solid = 2;
int state; // Current state (debug)

// Elements
final int air = 0; // Gas
final int water = 1; // Liquid
final int acid = 2; // Liquid
final int dirt = 3; // Solid
final int rock = 4; // Solid
int element; // Current element (debug)

// The game
Game game;

// The UI font
PFont font;

void setup() {
  println("setup");
  size(canvasX, canvasY);
  background(255);
  
  // Load in the UI font
  font = createFont("silkscreen.ttf", 100, false);
  textFont(font, 100); // 32 should be the max size the font is used
    
  // Set window title
  frame.setTitle("Pixel Plumber by Jack Cruden");
  
  // Set current element (debug)
  state = liquid;
  element = water;
 
  // Start the game
  game = new Game();
}

void draw() {
  background(255);
  
  // Generate new level (debug)
  //if (frameCount % 70 == 0) game = new Game();
  
  // Set window title
  frame.setTitle("Pixel Plumber by Jack Cruden - FPS: " + round(frameRate));
    
  // Mouse drag
//  if (mousePressed) {
//    for (int i = 0; i < numX; i++) {
//      for (int j = 0; j < numY; j++) {
//        Pixel p = game.level.ps[i][j];
//        if (mouseX > p.pos.x && mouseX < p.pos.x + size && mouseY > p.pos.y && mouseY < p.pos.y + size) {
//          if (i-penSize/2 > 0 && i+penSize/2 < numX && j-penSize/2 > 0 && j+penSize/2 < numY) {
//            for (int pI = -penSize/2; pI < penSize/2; pI++) {
//              for (int pJ = -penSize/2; pJ < penSize/2; pJ++) {
//                if (game.level.ps[i+pI][j+pJ].e != rock) {
//                  game.level.ps[i+pI][j+pJ].s = state;
//                  game.level.ps[i+pI][j+pJ].e = element;
//                }
//              }
//            }
//          }
//        }
//      }
//    }
//  }

  // Update and draw game
  game.draw();
}

void keyPressed() {
//  // Elements (debug)
//  switch(key) {
//    case '1':
//      state = gas;
//      element = air;
//      break;
//    case '2':
//      state = liquid;
//      element = water;
//      break;
//    case '3':
//      state = liquid;
//      element = acid;
//      break;
//    case '4':
//      state = solid;
//      element = dirt;
//      break;
//    case '5':
//      state = solid;
//      element = rock;
//      break;
//  }
  
  game.keyPressed(key);
}

// To detect if key has been held down
void keyReleased()
{ 
  game.keyReleased(key);
}

// To detect if the player made a shot
void mousePressed() {
  game.mousePressed(mouseButton);
}

// For debug
// At position pt, draw a vector v, a of length len
void drawVector(PVector pt, PVector v, float len) {
  pushStyle();
  
  // Set style
  strokeWeight(3);
  
  // Draw the line and arrow
  pushMatrix();
    translate(pt.x, pt.y);
    rotate(v.heading());
    line(0,0,len,0);
    triangle(.85*len, max(-.15*len,-10), len, 0, .85 * len, min(.15*len,10));
  popMatrix();
  
  popStyle();
}

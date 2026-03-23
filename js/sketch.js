// Jack Cruden
// MDDN242: Project 3
// Victoria University of Wellington

// Setup variables
const canvasX = 1035;
const canvasY = 780;
const gridSize = 15; // Size of pixels
const numX = Math.floor(canvasX / gridSize);
const numY = Math.floor(canvasY / gridSize);
const penSize = 4;

// States
const GAS = 0;
const LIQUID = 1;
const SOLID = 2;

// Elements
const AIR = 0;
const WATER = 1;
const ACID = 2;
const DIRT = 3;
const ROCK = 4;

// The game
let game;

// The UI font
let font;

function preload() {
  font = loadFont('Pixel_Plumber/data/silkscreen.ttf');
}

function setup() {
  createCanvas(canvasX, canvasY);
  background(255);
  textFont(font, 100);

  document.title = 'Pixel Plumber by Jack Cruden';

  game = new Game();
}

function draw() {
  background(255);

  document.title = 'Pixel Plumber by Jack Cruden - FPS: ' + round(frameRate());

  game.draw();
}

function keyPressed() {
  game.keyPressed(key);
}

function keyReleased() {
  game.keyReleased(key);
}

function mousePressed() {
  game.mousePressed(mouseButton);
}

// For debug
function drawVector(pt, v, len) {
  push();
  strokeWeight(3);

  push();
  translate(pt.x, pt.y);
  rotate(v.heading());
  line(0, 0, len, 0);
  triangle(0.85 * len, max(-0.15 * len, -10), len, 0, 0.85 * len, min(0.15 * len, 10));
  pop();

  pop();
}

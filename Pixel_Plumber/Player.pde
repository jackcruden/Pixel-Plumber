class Player {
  PVector pos; // Position
  PVector dir; // Direction
  PVector vel; // Velocity
  float maxSpeed = 1.4;
  float distance = 25; // Distance when rebound
  float mult = 0.4; // Rebound
  
  Player(PVector pos) {
    this.pos = pos;
    this.vel = new PVector(0, 0);
    this.dir = new PVector(0, 0);
  }
  
  Player(float x, float y) {
    this(new PVector(x, y));
  }
  
  void move() {
    move(new PVector(0, 0));
  }
  
  void move(PVector mvmt) {
    // Add mvmt to vel
    vel.add(mvmt);
        
    // Move the player
    vel.mult(0.94); // Friction and simple mass
    pos.add(vel);
  }
  
  // Check player collision
  void collision(Pixel[][] p) {
    for (int i = 0; i < p.length; i++) {
      for (int j = 0; j < p[0].length; j++) {
        float actualDist = dist(i*size+size/2, j*size+size/2, pos.x, pos.y);
        if (p[i][j].s == solid) {
          if (actualDist < distance) {
            // Vector from collision Pixel to Player
            PVector rebound = new PVector(pos.x - i*size, pos.y - j*size);
            rebound.normalize();
            rebound.mult(1);
            vel.normalize();
            vel.add(rebound);
          }
        }
      }
    }
  }
    
  void draw() {
    move();
    
    // Ensure maxSpeed
    if (vel.mag() > maxSpeed) {
      vel.normalize();
      vel.mult(maxSpeed);
    }
    
    // Alter the direction
    PVector newDir = new PVector(mouseX - pos.x, mouseY - pos.y);
    newDir.normalize();
    newDir.mult(0.1); // More natural direction changing
    dir.add(newDir);
    dir.normalize();
    
    // Check if still on canvas
    if (pos.x < 0) pos.x = width;
    if (pos.x > width) pos.x = 0;
    if (pos.y < 0) pos.y = height;
    if (pos.y > height) pos.y = 0;
    
    // Style
    pushStyle();
    noStroke();
    fill(0);
    
    // Move canvas and draw player
    pushMatrix();
      translate(pos.x, pos.y);
      rotate(dir.heading());
      rect(-size, -size, size*2, size*2);
      rect(size, -size/4, size/2, size/2);
    popMatrix();
        
    // Draw velocity vector
//    pushStyle();
//    stroke(100,100,255); // Blue
//    fill(100,100,255); // Blue
//    drawVector(pos, vel, vel.mag() * 10);
//    popStyle();
    
    // Restore previous style
    popStyle();
  }
}

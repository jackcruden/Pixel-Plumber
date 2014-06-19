class Shot {
  PVector pos; // Position
  PVector vel; // Velocity
  boolean active;
  int type = LEFT;
  
  Shot(int type) {
    this.pos = new PVector(game.player.pos.x, game.player.pos.y);
    this.vel = new PVector(mouseX - game.player.pos.x, mouseY - game.player.pos.y);
    this.type = type;
    active = true;
    
    this.vel.normalize();
    this.vel.mult(10); // Speed
    
    this.pos.add(this.vel);
  }
    
  void draw() {
    pushStyle();
    
    // Move the shot
    this.pos.add(this.vel);
    
    // Style
    fill(0);
    noStroke();
    
    // Move canvas and draw shot
    pushMatrix();
      translate(pos.x, pos.y);
      rotate(vel.heading());
      rect(-size/4, -size/4, size/2, size/2);
    popMatrix();
    
    popStyle();
  }
}

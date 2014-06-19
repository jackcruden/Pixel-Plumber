class Pixel {
  PVector pos;
  int s; // 0 = gas, 1 = liquid, 2 = solid
  int e; // Element
  
  Pixel(PVector pos, int s, int e) {
    this.pos = pos;
    this.s = s;
    this.e = e;
  }
  
  Pixel(PVector pos) {
    this(pos, air, 0);
  }
  
  void draw() {
    pushStyle();
    
    // Style
    noStroke();
    strokeWeight(0.001);
    switch(e) {
      case air:
        fill(255, 20);
        break;
      case water:
        fill(20, 150, 250);
        break;
      case acid:
        fill(140, 180, 100);
        break;
      case dirt:
        //stroke(250);
        fill(180, 150, 80);
        break;
      case rock:
        stroke(250);
        fill(100);
        break;
    }
    
    // Draw
    rect(pos.x, pos.y, size, size);
    //ellipse(pos.x + size/2-2, pos.y + size/2-2, size+4, size+4);
    
    popStyle();
  }
  
  boolean empty() {
    if (s == gas) return true;
    return false;
  }
}

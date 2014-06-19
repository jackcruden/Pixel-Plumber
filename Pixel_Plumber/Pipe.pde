class Pipe {
  int x;
  int y;
  boolean top; // Is pipe at top or bottom
  boolean active; // If pipe has touched water
  
  Pipe(int x, int y, boolean intake, boolean top) {
    this.x = x;
    this.y = y;
    this.top = top;
    this.active = false;
  }

  void draw() {
    pushStyle();
    
    fill(240, 120, 40);
    if (!top && !active) {
      fill(200);
    }
    strokeWeight(size/2);
    stroke(100);
    
    rect(x*size, y*size, size*6, size*2);
    if (top) {
      rect((x+1)*size, (y-3)*size, size*4, size*3);
    } else {
      rect((x+1)*size, (y+2)*size, size*4, size*3);
    }
    
    popStyle();
  }
}

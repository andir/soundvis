#version 450
in vec2 position;
out vec4 v_position;


void main()
{
   float x = position.x;

   gl_Position = vec4(position, 1., 1.);
   v_position = gl_Position;
}


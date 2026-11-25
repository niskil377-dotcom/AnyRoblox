using System;
using System.IO;
using System.Collections.Generic;
using System.Globalization;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Media.Media3D;
using System.Windows.Input;

namespace ArmaStudio {
    sealed class AxesIndicator : FrameworkElement {
        public Vector3D Right, Up, Toward;
        protected override void OnRender(DrawingContext dc) {
            var axes=new[]{
                new {Name="X",Dir=new Vector3D(1,0,0),Color=Colors.IndianRed},
                new {Name="Y",Dir=new Vector3D(0,0,-1),Color=Colors.YellowGreen},
                new {Name="Z",Dir=new Vector3D(0,1,0),Color=Colors.DodgerBlue}
            };
            var center=new Point(42,42);
            foreach(var a in System.Linq.Enumerable.OrderBy(axes,x=>Vector3D.DotProduct(x.Dir,Toward))){
                var brush=new SolidColorBrush(a.Color);
                var tip=center+new Vector(28*Vector3D.DotProduct(a.Dir,Right),-28*Vector3D.DotProduct(a.Dir,Up));
                dc.DrawLine(new Pen(brush,2),center,tip);
                dc.DrawEllipse(brush,null,tip,8,8);
                var text=new FormattedText(a.Name,CultureInfo.InvariantCulture,FlowDirection.LeftToRight,new Typeface("Segoe UI"),10,Brushes.Black,VisualTreeHelper.GetDpi(this).PixelsPerDip);
                dc.DrawText(text,new Point(tip.X-text.Width/2,tip.Y-text.Height/2));
            }
        }
    }

    public class OrbitView : Grid {
        readonly Viewport3D view = new Viewport3D();
        readonly PerspectiveCamera camera = new PerspectiveCamera();
        readonly ModelVisual3D subject = new ModelVisual3D();
        bool gridVisible=true;
        double gridStep=1;
        Point3D gridCenter;
        Model3DGroup sourceModel;
        bool solid;
        readonly AxesIndicator axes = new AxesIndicator {Width=84, Height=84, HorizontalAlignment=HorizontalAlignment.Right, VerticalAlignment=VerticalAlignment.Top, Margin=new Thickness(8), IsHitTestVisible=false};

        Point previous;
        Point3D pivot;
        Rect3D bounds;
        double yaw, pitch, distance, radius;
        public OrbitView() {
            Background = new SolidColorBrush(Color.FromRgb(61,61,61));
            ClipToBounds = true;
            Children.Add(view);
            Children.Add(axes);
            view.Camera = camera;
            camera.FieldOfView = 40;
            var lights = new Model3DGroup();
            lights.Children.Add(new AmbientLight(Color.FromRgb(84,84,84)));
            lights.Children.Add(new DirectionalLight(Color.FromRgb(210,209,205),new Vector3D(-.35,-1,-.55)));
            lights.Children.Add(new DirectionalLight(Color.FromRgb(87,95,106),new Vector3D(1,-.15,.65)));
            view.Children.Add(new ModelVisual3D { Content=lights });
            view.Children.Add(subject);
            SetGrid(true);Clear();
            MouseWheel += (s,e)=>{ Zoom(e.Delta); e.Handled=true; };
            MouseDown += (s,e)=>{
                if(e.ChangedButton!=MouseButton.Middle)return;
                previous=e.GetPosition(this);
                CaptureMouse(); e.Handled=true;
            };
            MouseUp += (s,e)=>{if(e.ChangedButton==MouseButton.Middle)ReleaseMouseCapture();};
            MouseMove += (s,e)=>{
                if(!IsMouseCaptured || e.MiddleButton!=MouseButtonState.Pressed)return;
                Point now=e.GetPosition(this);
                Drag(now.X-previous.X,now.Y-previous.Y,(Keyboard.Modifiers & ModifierKeys.Shift)!=0);
                previous=now;
            };
        }
        public void SetGrid(bool visible){gridVisible=visible;InvalidateVisual();}
        protected override void OnRender(DrawingContext dc){
            base.OnRender(dc);
            if(!gridVisible || ActualWidth<1 || radius<=0)return;
            var forward=camera.LookDirection;
            var right=Vector3D.CrossProduct(forward,camera.UpDirection);right.Normalize();
            var up=Vector3D.CrossProduct(right,forward);
            double scale=ActualWidth/(2*Math.Tan(camera.FieldOfView*Math.PI/360));
            const int extent=50;
            var pens=new[]{new Pen(new SolidColorBrush(Color.FromRgb(72,72,72)),0.7),
                new Pen(new SolidColorBrush(Color.FromRgb(83,83,83)),0.9),
                new Pen(new SolidColorBrush(Color.FromRgb(151,69,76)),1.1),
                new Pen(new SolidColorBrush(Color.FromRgb(100,135,69)),1.1)};
            for(int i=-extent;i<=extent;i++)for(int axis=0;axis<2;axis++){
                var along=axis==0?new Vector3D(1,0,0):new Vector3D(0,0,1);
                var across=axis==0?new Vector3D(0,0,1):new Vector3D(1,0,0);
                var center=gridCenter+i*gridStep*across-camera.Position;
                var a=center-extent*gridStep*along;
                var b=center+extent*gridStep*along;
                double za=Vector3D.DotProduct(a,forward),zb=Vector3D.DotProduct(b,forward);
                double near=Math.Max(gridStep*.25,camera.NearPlaneDistance);
                if(za<=near && zb<=near)continue;
                if(za<near){a+=(b-a)*((near-za)/(zb-za));za=near;}
                if(zb<near){b+=(a-b)*((near-zb)/(za-zb));zb=near;}
                var p=new Point(ActualWidth/2+scale*Vector3D.DotProduct(a,right)/za,ActualHeight/2-scale*Vector3D.DotProduct(a,up)/za);
                var q=new Point(ActualWidth/2+scale*Vector3D.DotProduct(b,right)/zb,ActualHeight/2-scale*Vector3D.DotProduct(b,up)/zb);
                dc.DrawLine(pens[i==0?2+axis:i%5==0?1:0],p,q);
            }
        }
        public void SetSolid(bool enabled){
            solid=enabled;
            if(!enabled || sourceModel==null){subject.Content=sourceModel;return;}
            var material=new DiffuseMaterial(new SolidColorBrush(Color.FromRgb(168,170,175)));
            var preview=new Model3DGroup();
            foreach(GeometryModel3D part in sourceModel.Children)
                preview.Children.Add(new GeometryModel3D(part.Geometry,material){BackMaterial=material,Transform=part.Transform});
            preview.Freeze();subject.Content=preview;
        }
        public void ShowModel(Model3DGroup model) {
            sourceModel=model;SetSolid(solid);
            bounds=model.Bounds;
            Frame();
        }
        public void Clear() {
            sourceModel=null;subject.Content=null; bounds=Rect3D.Empty; Frame();
        }
        public void Frame() {
            if(bounds.IsEmpty){
                pivot=new Point3D();radius=4;distance=14;yaw=.68;pitch=.49;
                gridCenter=new Point3D();gridStep=1;UpdateCamera();return;
            }
            pivot=new Point3D(bounds.X+bounds.SizeX/2,bounds.Y+bounds.SizeY/2,bounds.Z+bounds.SizeZ/2);
            radius=Math.Max(1e-8,new Vector3D(bounds.SizeX,bounds.SizeY,bounds.SizeZ).Length/2);
            gridStep=radius/4;
            gridCenter=new Point3D(pivot.X,bounds.Y-radius*.004,pivot.Z);
            double aspect=ActualWidth/Math.Max(1,ActualHeight);
            yaw=0.32; pitch=0.22;
            if(bounds.SizeY<Math.Min(bounds.SizeX,bounds.SizeZ)*0.1)pitch=1.22;
            if(bounds.SizeX<Math.Min(bounds.SizeY,bounds.SizeZ)*0.1)yaw=1.3;
            var direction=Direction();
            var right=Vector3D.CrossProduct(new Vector3D(0,1,0),direction); right.Normalize();
            var up=Vector3D.CrossProduct(direction,right);
            double tangent=Math.Tan(camera.FieldOfView*Math.PI/360);
            distance=radius;
            for(int i=0;i<8;i++) {
                var corner=new Vector3D(((i&1)==0?-0.5:0.5)*bounds.SizeX,
                    ((i&2)==0?-0.5:0.5)*bounds.SizeY,((i&4)==0?-0.5:0.5)*bounds.SizeZ);
                double extent=Math.Max(Math.Abs(Vector3D.DotProduct(corner,right)),
                    Math.Abs(Vector3D.DotProduct(corner,up))*Math.Max(0.1,aspect));
                distance=Math.Max(distance,Vector3D.DotProduct(corner,direction)+1.35*extent/tangent);
            }
            UpdateCamera();
        }
        public void Zoom(double delta) {
            if(radius<=0)return;
            distance=Math.Max(radius*0.025,Math.Min(radius*150,distance*Math.Pow(0.84,delta/120)));
            UpdateCamera();
        }
        public void Drag(double dx,double dy,bool pan) {
            if(radius<=0)return;
            if(pan) {
                var forward=camera.LookDirection; forward.Normalize();
                var right=Vector3D.CrossProduct(forward,new Vector3D(0,1,0));right.Normalize();
                var up=Vector3D.CrossProduct(right,forward);
                double scale=2*distance*Math.Tan(camera.FieldOfView*Math.PI/360)/Math.Max(1,ActualWidth);
                pivot+=scale*(-dx*right+dy*up);
            } else {
                yaw-=dx*0.009;
                pitch=Math.Max(-1.55,Math.Min(1.55,pitch+dy*0.009));
            }
            UpdateCamera();
        }
        Vector3D Direction() {
            return new Vector3D(Math.Sin(yaw)*Math.Cos(pitch),Math.Sin(pitch),Math.Cos(yaw)*Math.Cos(pitch));
        }
        void UpdateCamera() {
            var offset=Direction();
            camera.Position=pivot+distance*offset;
            camera.LookDirection=-offset;
            camera.UpDirection=new Vector3D(0,1,0);
            camera.NearPlaneDistance=Math.Max(radius*0.0001,distance-radius*3);
            camera.FarPlaneDistance=distance+radius*1000;
            var right=Vector3D.CrossProduct(camera.LookDirection,camera.UpDirection);right.Normalize();
            axes.Right=right;axes.Up=Vector3D.CrossProduct(offset,right);axes.Toward=offset;axes.InvalidateVisual();InvalidateVisual();
        }
    }

    public class LoadedModel {
        public Model3DGroup Group = new Model3DGroup();
        public int Vertices, Faces;
        public string Notice="";
    }
    public static class ObjReader {
        static double Number(string s) {
            double n=double.Parse(s,CultureInfo.InvariantCulture);
            if(double.IsNaN(n)||double.IsInfinity(n))throw new InvalidDataException("Invalid coordinate.");
            return n;
        }
        static int Index(string s,int count) {
            int i=int.Parse(s,CultureInfo.InvariantCulture);
            i=i<0?count+i:i-1;
            if(i<0||i>=count)throw new InvalidDataException("OBJ index out of range.");
            return i;
        }
        static string Local(string root,string file) {
            string path=Path.GetFullPath(Path.Combine(root,file));
            if(!path.StartsWith(Path.GetFullPath(root).TrimEnd('\\')+"\\",StringComparison.OrdinalIgnoreCase))
                throw new InvalidDataException("Resource is outside the model folder.");
            return path;
        }
        static Material Surface(Brush brush) { return new DiffuseMaterial(brush); }
        static void ReadMaterials(string path,Dictionary<string,Material> materials,LoadedModel result) {
            if(!File.Exists(path)){result.Notice="MTL file missing.";return;}
            string current="";
            foreach(string raw in File.ReadLines(path)) {
                string line=raw.Trim();
                if(line.StartsWith("newmtl ")) current=line.Substring(7).Trim();
                if(line.StartsWith("Kd ")) {
                    string[] p=line.Split((char[])null,StringSplitOptions.RemoveEmptyEntries);
                    materials[current]=Surface(new SolidColorBrush(Color.FromScRgb(1,(float)Number(p[1]),(float)Number(p[2]),(float)Number(p[3]))));
                }
                if(line.StartsWith("map_Kd ")) {
                    string image=Local(Path.GetDirectoryName(path),line.Substring(7).Trim());
                    if(!File.Exists(image)){result.Notice="Texture missing.";continue;}
                    var bitmap=new BitmapImage();
                    using(var file=File.OpenRead(image)){
                        bitmap.BeginInit();bitmap.CacheOption=BitmapCacheOption.OnLoad;bitmap.StreamSource=file;bitmap.EndInit();
                    }
                    bitmap.Freeze();
                    materials[current]=Surface(new ImageBrush(bitmap){TileMode=TileMode.Tile,ViewportUnits=BrushMappingMode.Absolute,Viewport=new Rect(0,0,1,1)});
                }
            }
        }
        public static LoadedModel Read(string path) {
            var result=new LoadedModel();
            var points=new List<Point3D>();
            var normals=new List<Vector3D>();
            var uv=new List<Point>();
            var materials=new Dictionary<string,Material>();
            var meshes=new Dictionary<string,MeshGeometry3D>();
            string current="";
            foreach(string raw in File.ReadLines(path)) {
                string line=raw.Split('#')[0].Trim();
                if(line.Length==0)continue;
                string[] p=line.Split((char[])null,StringSplitOptions.RemoveEmptyEntries);
                switch(p[0]) {
                case "v": points.Add(new Point3D(Number(p[1]),Number(p[2]),Number(p[3])));break;
                case "vn": normals.Add(new Vector3D(Number(p[1]),Number(p[2]),Number(p[3])));break;
                case "vt": uv.Add(new Point(Number(p[1]),1-Number(p[2])));break;
                case "mtllib": ReadMaterials(Local(Path.GetDirectoryName(path),line.Substring(6).Trim()),materials,result);break;
                case "usemtl": current=line.Substring(6).Trim();break;
                case "f":
                    if(p.Length<4)throw new InvalidDataException("Incomplete face.");
                    MeshGeometry3D mesh;
                    if(!meshes.TryGetValue(current,out mesh))meshes[current]=mesh=new MeshGeometry3D();
                    int start=mesh.Positions.Count;
                    for(int i=1;i<p.Length;i++) {
                        var c=p[i].Split('/');
                        mesh.Positions.Add(points[Index(c[0],points.Count)]);
                        mesh.TextureCoordinates.Add(c.Length>1&&c[1]!=""?uv[Index(c[1],uv.Count)]:new Point());
                        if(c.Length>2&&c[2]!="")mesh.Normals.Add(normals[Index(c[2],normals.Count)]);
                        if(i>=3){mesh.TriangleIndices.Add(start);mesh.TriangleIndices.Add(start+i-2);mesh.TriangleIndices.Add(start+i-1);}
                    }
                    result.Faces++;break;
                }
            }
            foreach(var part in meshes) {
                Material mat;
                if(!materials.TryGetValue(part.Key,out mat))mat=Surface(Brushes.LightGray);
                if(part.Value.Normals.Count!=part.Value.Positions.Count)part.Value.Normals.Clear();
                result.Group.Children.Add(new GeometryModel3D(part.Value,mat){BackMaterial=mat});
            }
            if(result.Faces==0)throw new InvalidDataException("No faces to display.");
            result.Vertices=points.Count;
            result.Group.Freeze();
            return result;
        }
    }
}

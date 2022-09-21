#Metrics results
#WARNING: 12698 points with same coordinates found
#Imported intrinsic resoluiton: 1023
#Peak distance for PSNR: 1023
#Point cloud sizes for org version, dec version, and the scaling ratio: 765821, 688562, 0.899116
#1. Use infile1 (A) as reference, loop over A, use normals on B. (A->B).
#   mse1      (p2point): 0.739141
#   mse1,PSNR (p2point): 66.2814
#   c[0],    1         : 0.00193168
#   c[1],    1         : 0.000488816
#   c[2],    1         : 0.000555591
#   c[0],PSNR1         : 27.1406
#   c[1],PSNR1         : 33.1085
#   c[2],PSNR1         : 32.5524
#2. Use infile2 (B) as reference, loop over B, use normals on A. (B->A).
#   mse2      (p2point): 0.655312
#   mse2,PSNR (p2point): 66.8042
#   c[0],    2         : 0.00175507
#   c[1],    2         : 0.000479594
#   c[2],    2         : 0.000541722'


#   c[0],PSNR2         : 27.5571
#   c[1],PSNR2         : 33.1913
#   c[2],PSNR2         : 32.6622
#3. Final (symmetric).
#   mseF      (p2point): 0.739141
#   mseF,PSNR (p2point): 66.2814
#   mseF      (p2plane): 0
#   mseF,PSNR (p2plane): 0
#   c[0],    F         : 0.00193168
#   c[1],    F         : 0.000488816
#   c[2],    F         : 0.000555591
#   c[0],PSNRF         : 27.1406
#   c[1],PSNRF         : 33.1085
#   c[2],PSNRF         : 32.5524
#metric:Processing time (wall): 28.026 s
#metric:Processing time (user.self): 0 s
#metric:Processing time (user.children): 0 s
PccAppMetrics --uncompressedDataPath=./test_files/metrics/original.ply --reconstructedDataPath=./test_files/metrics/reconstructed.ply --frameCount 1
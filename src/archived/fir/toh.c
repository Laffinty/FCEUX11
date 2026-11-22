/* FCE Ultra - NES/Famicom Emulator
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301  USA
 */
#include <stdio.h>

#define MAX        512

main()
{
 char buf[256];
 int count=0;

 while(fgets(buf,256,stdin)>0)
 {
  double p;

  if(sscanf(buf,"%lf",&p)==1)
  {
   p*=65536*16;
   printf("%ld,\n",(long)p);
   count++;
   if(count==MAX) break;
  }

 }

}

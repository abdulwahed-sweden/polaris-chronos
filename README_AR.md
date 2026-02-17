<h1 align="center">
  بولاريس كرونوس
</h1>

<p align="center">
  <strong>محرك مواقيت الصلاة العالمي</strong>
</p>

<p align="center">
  <a href="#"><img src="https://img.shields.io/badge/Rust-2021_Edition-DEA584?logo=rust&logoColor=white" alt="Rust"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="MIT License"></a>
  <a href="#"><img src="https://img.shields.io/badge/Tests-96_passing-brightgreen" alt="Tests"></a>
  <a href="#"><img src="https://img.shields.io/badge/Version-1.0.0-purple" alt="Version"></a>
  <a href="https://huggingface.co/spaces/abdulwahed-sweden/polaris-chronos"><img src="https://img.shields.io/badge/تجربة_مباشرة-HF_Spaces-yellow?logo=huggingface" alt="Live Demo"></a>
</p>

<p align="center">
  محرك فلكي عالي الدقة مكتوب بلغة Rust<br>
  يحسب أوقات الصلاة لأي مكان على وجه الأرض — بما في ذلك المناطق القطبية<br>
  يتضمن لوحة تحكم ويب وواجهة برمجية RESTful
</p>

<p align="center">
  <a href="README.md"><strong>English</strong></a>&nbsp;&nbsp;&nbsp;|&nbsp;&nbsp;&nbsp;<a href="README_AR.md"><strong>العربية</strong></a>
</p>

<p align="center">
  <strong>تجربة مباشرة:</strong> <a href="https://abdulwahed-sweden-polaris-chronos.hf.space">abdulwahed-sweden-polaris-chronos.hf.space</a>
</p>

<br>

<div dir="rtl">

<table>
<tr>
<td>

<p align="right">
تطبيقات المواقيت التقليدية تفشل فوق خط عرض 65° شمالاً.
الشمس لا تغرب — فلا يوجد مغرب. الشمس لا تشرق — فلا يوجد فجر.
</p>

<p align="right">
<strong>بولاريس لا يفشل.</strong>
يعالج الشمس كموجة زاوية مستمرة، وينتج جدولاً كاملاً وشفافاً — في كل مكان، كل يوم، كل حالة استثنائية.
</p>

</td>
</tr>
</table>

<br>

<h2 align="right">المشكلة</h2>

<p align="right">
تطبيقات المواقيت التقليدية تعتمد على فكرة واحدة:
</p>

<p align="right"><em>
"المغرب = لحظة غروب الشمس"
</em></p>

<p align="right">
لكن في أماكن مثل شمال النرويج والسويد وألاسكا، هناك ظواهر لا تتعامل معها هذه التطبيقات:
</p>

<br>

<table>
<thead>
<tr>
<th align="right">النتيجة في التطبيقات التقليدية</th>
<th align="right">ماذا يحدث</th>
<th align="right">الظاهرة</th>
</tr>
</thead>
<tbody>
<tr>
<td align="right">لا يوجد مغرب ولا عشاء — خطأ أو فراغ</td>
<td align="right">الشمس لا تغرب لأسابيع</td>
<td align="right"><strong>شمس منتصف الليل</strong></td>
</tr>
<tr>
<td align="right">لا يوجد فجر ولا شروق — خطأ أو فراغ</td>
<td align="right">الشمس لا تشرق لأسابيع</td>
<td align="right"><strong>الليل القطبي</strong></td>
</tr>
</tbody>
</table>

<br>

<p align="right">
بولاريس لا يتوقف عند هذه الحالات. <strong>يحلّها.</strong>
</p>

<br>

<hr>

<br>

<h2 align="right">الفكرة الأساسية</h2>

<p align="right">
بولاريس لا ينظر إلى الشمس كجسم "مرئي أو غير مرئي" فقط.
</p>

<p align="right">
بل يتعامل معها كـ <strong>حركة زاوية مستمرة</strong> — موجة رياضية لا تتوقف
حتى لو كانت الشمس فوق الأفق أو تحته طوال اليوم.
</p>

<br>

<p align="right">هذا يعني:</p>

<p align="right">
حتى لو الشمس لم تغرب — <strong>النظام يعرف أين "كان يجب" أن تغرب</strong>
<br><br>
حتى لو لم يكن هناك شفق — <strong>النظام يحسب متى "كان سيظهر" الشفق</strong>
</p>

<br>

<p align="right">
النتيجة: <strong>جدول صلاة كامل، لكل يوم، في أي مكان.</strong>
</p>

<br>

<hr>

<br>

<h2 align="right">طرق الحساب الثلاث</h2>

<p align="right">
بولاريس يختار الطريقة تلقائياً حسب الحالة الفلكية:
</p>

<br>

<table>
<thead>
<tr>
<th align="right">كيف تعمل</th>
<th align="right">متى تُستخدم</th>
<th align="center">الثقة</th>
<th align="right">الطريقة</th>
</tr>
</thead>
<tbody>
<tr>
<td align="right">حساب فلكي مباشر — نفس المنهج المستخدم في معظم التطبيقات</td>
<td align="right">الشمس تشرق وتغرب بشكل طبيعي</td>
<td align="center"><code>1.0</code></td>
<td align="right"><strong>Standard</strong></td>
</tr>
<tr>
<td align="right">اشتقاق الوقت من أدنى نقطة في الموجة — "منتصف الليل الفلكي"</td>
<td align="right">لا توجد زاوية كافية للفجر أو العشاء</td>
<td align="center"><code>0.7</code></td>
<td align="right"><strong>Virtual</strong></td>
</tr>
<tr>
<td align="right">إسقاط نسب اليوم من خط عرض معتدل (~45°–55°) على موقعك</td>
<td align="right">لا يوجد شروق أو غروب إطلاقاً</td>
<td align="center"><code>0.5</code></td>
<td align="right"><strong>Projected</strong></td>
</tr>
</tbody>
</table>

<br>

<blockquote>
<p align="right">
طريقة الإسقاط (Projected) قريبة من المفهوم الفقهي المعروف <strong>"التقدير بأقرب البلاد"</strong>
— لكن بولاريس يحوّلها إلى نموذج رياضي دقيق يعمل تلقائياً.
</p>
</blockquote>

<br>

<hr>

<br>

<h2 align="right">مثال حقيقي — ترومسو، النرويج — 21 يونيو 2026</h2>

<blockquote>
<p align="right">
<strong>شمس منتصف الليل.</strong>
الشمس لم تغرب طوال اليوم. أدنى ارتفاع لها: <strong>+3.1°</strong> فوق الأفق.
</p>
</blockquote>

<br>

<pre dir="ltr" align="left"><code>polaris Tromso --date 2026-06-21</code></pre>

<br>

<table>
<thead>
<tr>
<th align="right">التفسير</th>
<th align="center">الثقة</th>
<th align="center">الطريقة</th>
<th align="center">الوقت</th>
<th align="right">الصلاة</th>
</tr>
</thead>
<tbody>
<tr>
<td align="right">مشتق من أدنى نقطة في الموجة الشمسية</td>
<td align="center">0.70</td>
<td align="center">Virtual</td>
<td align="center"><code>00:46</code> +1 يوم</td>
<td align="right"><strong>الفجر</strong></td>
</tr>
<tr>
<td align="right">إسقاط من خط عرض مرجعي 54.7°</td>
<td align="center">0.50</td>
<td align="center">Projected</td>
<td align="center"><code>04:07</code></td>
<td align="right"><strong>الشروق</strong></td>
</tr>
<tr>
<td align="right">ذروة الشمس — حساب فلكي مباشر</td>
<td align="center">1.00</td>
<td align="center">Standard</td>
<td align="center"><code>12:46</code></td>
<td align="right"><strong>الظهر</strong></td>
</tr>
<tr>
<td align="right">نسبة طول الظل — حساب فلكي مباشر</td>
<td align="center">1.00</td>
<td align="center">Standard</td>
<td align="center"><code>17:57</code></td>
<td align="right"><strong>العصر</strong></td>
</tr>
<tr>
<td align="right">إسقاط من خط عرض مرجعي 54.7°</td>
<td align="center">0.50</td>
<td align="center">Projected</td>
<td align="center"><code>21:24</code></td>
<td align="right"><strong>المغرب</strong></td>
</tr>
<tr>
<td align="right">مشتق من أدنى نقطة في الموجة الشمسية</td>
<td align="center">0.70</td>
<td align="center">Virtual</td>
<td align="center"><code>00:46</code> +1 يوم</td>
<td align="right"><strong>العشاء</strong></td>
</tr>
</tbody>
</table>

<br>

<hr>

<br>

<h2 align="right">لوحة تحكم الويب</h2>

<p align="right">
بولاريس يتضمن لوحة تحكم ويب مدمجة تشمل:
</p>

<p align="right">
<strong>تحديد الموقع تلقائياً</strong> عبر GPS — يجد أقرب مدينة أو يستخدم الإحداثيات الدقيقة
<br>
<strong>عرض أسبوعي / شهري / يومي</strong> يبدأ دائماً من اليوم
<br>
<strong>ثلاثة أعمدة للتاريخ</strong> — اسم اليوم، التاريخ الميلادي، التاريخ الهجري
<br>
<strong>تمييز يوم الجمعة</strong> بلون أخضر فاتح
<br>
<strong>لوحة الصلاة الحالية</strong> مع العد التنازلي للصلاة التالية
<br>
<strong>مخطط الأفق</strong> — رسم بياني SVG لمسار الشمس
<br>
<strong>بحث المدن</strong> مع الإكمال التلقائي وخيارات التوضيح
<br>
<strong>توثيق API</strong> — صفحة مطورين مدمجة على <code>/docs</code>
</p>

<br>

<p align="right">
<strong>تجربة مباشرة:</strong> <a href="https://abdulwahed-sweden-polaris-chronos.hf.space">abdulwahed-sweden-polaris-chronos.hf.space</a>
</p>

<br>

<hr>

<br>

<h2 align="right">ما الفرق بين بولاريس والتطبيقات الأخرى؟</h2>

<br>

<table>
<thead>
<tr>
<th align="center">بولاريس</th>
<th align="center">التطبيقات التقليدية</th>
<th align="right">المقارنة</th>
</tr>
</thead>
<tbody>
<tr>
<td align="center">جدول كامل دائماً</td>
<td align="center">يفشل أو يعطي نتائج خاطئة</td>
<td align="right">يعمل في المناطق القطبية</td>
</tr>
<tr>
<td align="center">كل وقت موسوم بطريقته</td>
<td align="center">نتيجة بدون تفسير</td>
<td align="right">يوضح طريقة الحساب</td>
</tr>
<tr>
<td align="center">درجة ثقة لكل وقت</td>
<td align="center">كل النتائج تبدو متساوية</td>
<td align="right">يميز الحقيقي من التقديري</td>
</tr>
<tr>
<td align="center">محاكاة موقع الشمس (SPA)</td>
<td align="center">معادلات مبسطة</td>
<td align="right">يعتمد على الفيزياء الفلكية</td>
</tr>
<tr>
<td align="center">قاعدة بيانات مدمجة + كاش</td>
<td align="center">يحتاج اتصال غالباً</td>
<td align="right">يعمل بدون إنترنت</td>
</tr>
</tbody>
</table>

<br>

<hr>

<br>

<h2 align="right">لماذا الثقة مهمة؟</h2>

<p align="right">
لأن <strong>الصدق أهم من الدقة الوهمية.</strong>
</p>

<p align="right">
عندما يعطيك تطبيق وقت المغرب في ترومسو صيفاً بدون أي تنبيه — فهو يكذب عليك.
الشمس لم تغرب أصلاً.
</p>

<p align="right">
بولاريس يقول لك بوضوح:
</p>

<blockquote>
<p align="right">
هذا الوقت <strong>تقديري</strong> (ثقة 0.5) — محسوب بالإسقاط من خط عرض معتدل، لأن الغروب الحقيقي لم يحدث.
</p>
</blockquote>

<p align="right">
هذه الشفافية ليست ضعفاً — بل هي <strong>أمانة علمية.</strong>
</p>

<br>

<hr>

<br>

<h2 align="right">النشر والتشغيل</h2>

<h3 align="right">التشغيل المحلي</h3>

<pre dir="ltr" align="left"><code>cargo build --release
./target/release/polaris server --port 3000
# افتح http://localhost:3000</code></pre>

<h3 align="right">Docker</h3>

<pre dir="ltr" align="left"><code>docker build -t polaris-chronos .
docker run -p 7860:7860 polaris-chronos
# افتح http://localhost:7860</code></pre>

<h3 align="right">Hugging Face Spaces</h3>

<p align="right">
المشروع منشور على Hugging Face Spaces مع Dockerfile جاهز للنشر التلقائي.
</p>

<br>

<hr>

<br>

<h2 align="right">دعم المدن الفلسطينية</h2>

<p align="right">
يتضمن بولاريس 34 مدينة مدمجة، منها 5 مدن فلسطينية:
</p>

<table>
<thead>
<tr>
<th align="right">المنطقة الزمنية</th>
<th align="right">الأسماء البديلة</th>
<th align="right">المدينة</th>
</tr>
</thead>
<tbody>
<tr>
<td align="right">Asia/Jerusalem</td>
<td align="right">القدس، al-quds</td>
<td align="right"><strong>القدس</strong></td>
</tr>
<tr>
<td align="right">Asia/Gaza</td>
<td align="right">غزة، ghazza</td>
<td align="right"><strong>غزة</strong></td>
</tr>
<tr>
<td align="right">Asia/Hebron</td>
<td align="right">—</td>
<td align="right"><strong>رام الله</strong></td>
</tr>
<tr>
<td align="right">Asia/Hebron</td>
<td align="right">الخليل، al-khalil</td>
<td align="right"><strong>الخليل</strong></td>
</tr>
<tr>
<td align="right">Asia/Hebron</td>
<td align="right">نابلس، nablous</td>
<td align="right"><strong>نابلس</strong></td>
</tr>
</tbody>
</table>

<br>

<pre dir="ltr" align="left"><code>polaris Gaza
  gaza — فلسطين
  Asia/Gaza (Local Time)
  31.50°N, 34.47°E</code></pre>

<br>

<hr>

<br>

<h2 align="right">مبادئ التصميم</h2>

<br>

<table>
<tbody>
<tr>
<td align="right">موقع الشمس يُحسب فلكياً — لا تقريب ولا ترميز ثابت</td>
<td align="right"><strong>الفيزياء أولاً</strong></td>
</tr>
<tr>
<td align="right">كل قيمة تشرح كيف تم اشتقاقها</td>
<td align="right"><strong>الشفافية</strong></td>
</tr>
<tr>
<td align="right">يعمل بنفس الطريقة من مكة (21° ش) إلى سفالبارد (78° ش)</td>
<td align="right"><strong>العالمية</strong></td>
</tr>
<tr>
<td align="right">نفس الإحداثيات + نفس التاريخ = نفس النتيجة دائماً</td>
<td align="right"><strong>الحتمية</strong></td>
</tr>
<tr>
<td align="right">عندما تنخفض الدقة — تنخفض درجة الثقة معها</td>
<td align="right"><strong>الصدق</strong></td>
</tr>
</tbody>
</table>

<br>

<hr>

<br>

<h2 align="right">الرخصة</h2>

<p align="right">MIT</p>

<br>

</div>

<p align="center">
<strong>للتوثيق التقني والتشغيل:</strong> <a href="README.md">README.md</a>
</p>

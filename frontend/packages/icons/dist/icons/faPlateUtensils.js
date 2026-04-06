var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPlateUtensils.js
var require_faPlateUtensils = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPlateUtensils.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "plate-utensils";
    var width = 640;
    var height = 512;
    var aliases = [58713];
    var unicode = "e43b";
    var svgPathData = "M40.7 16c4 .3 7.1 3.5 7.3 7.5L55.5 152l17.3 0L88.1 23.1c.5-4 3.9-7.1 7.9-7.1s7.5 3 7.9 7.1L119.1 152l17.3 0L144 23.5c.2-4 3.3-7.2 7.3-7.5s7.6 2.3 8.5 6.1l30.3 128.7c1.3 5.4 1.9 11 1.9 16.6c0 39.9-32.2 72.2-72 72.5l0 232c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-232c-39.8-.3-72-32.6-72-72.5c0-5.6 .6-11.2 1.9-16.6L32.2 22.2c.9-3.9 4.5-6.5 8.5-6.1zm572.4 1c2-.7 4-1 6.1-1c11.5 0 20.9 9.3 20.9 20.9L640 216l0 104 0 152c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-152-48 0c-17.7 0-32-14.3-32-32l0-128C512 59.2 596.2 23.2 613.1 17zM221.2 143.6L204.9 74.3C241.8 47.7 287.1 32 336 32c60.3 0 115 23.8 155.3 62.5C484.3 113.3 480 135 480 160l0 26.2C454.1 132.8 399.3 96 336 96c-44.9 0-85.5 18.5-114.6 48.3l-.2-.8zM336 480c-76.2 0-143.5-38.1-184-96.2l0-116.9c8.5-2.8 16.6-6.7 24-11.4l0 .6c0 88.4 71.6 160 160 160c68.1 0 126.2-42.5 149.3-102.4c9.2 21 29.3 36.2 53.2 38.2C502.6 427.6 425.4 480 336 480zM208 256a128 128 0 1 1 256 0 128 128 0 1 1 -256 0z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faPlateUtensils = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faPlateUtensils.js
var import_faPlateUtensils = __toESM(require_faPlateUtensils());
var export_faPlateUtensils = import_faPlateUtensils.definition;
export {
  export_faPlateUtensils as faPlateUtensils
};

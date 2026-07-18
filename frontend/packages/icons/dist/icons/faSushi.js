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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSushi.js
var require_faSushi = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSushi.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "sushi";
    var width = 576;
    var height = 512;
    var aliases = [127843, "nigiri"];
    var unicode = "e48a";
    var svgPathData = "M479.7 320l-73.8-24.5L506.3 125.6c24.8 26.7 42.1 60.7 48.5 98.8l12.5 75.2c4 23.8-18.8 43.2-41.7 35.6l-7-2.3L479.7 320s0 0 0 0zM393.5 66.1c32.9 4.8 63.2 18 88.7 37.5L374.8 285.2 323.1 268c-14.6-4.9-29.9-7-45.2-6.4L393.5 66.1zm-36-2.1L235.6 270.4 96.1 318.7 246.6 64l110.9 0zM21.3 224.4C36.6 132.2 116.1 64.6 209.5 64L49.4 334.8C27 341.5 5 322.3 8.9 298.9l12.4-74.4zM32 384l0-15c8.5 .7 17.5-.3 26.4-3.4l193-66.9c20-6.9 41.6-7.1 61.7-.4l205.5 68.2c8.6 2.9 17.2 3.8 25.5 3.1l0 14.3c0 35.3-28.7 64-64 64c-19.1 0-36.3-8.4-48-21.7c-11.7 13.3-28.9 21.7-48 21.7s-36.3-8.4-48-21.7c-11.7 13.3-28.9 21.7-48 21.7s-36.3-8.4-48-21.7c-11.7 13.3-28.9 21.7-48 21.7s-36.3-8.4-48-21.7C132.3 439.6 115.1 448 96 448c-35.3 0-64-28.7-64-64z";
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
    exports.faSushi = exports.definition;
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

// src/_entries/faSushi.js
var import_faSushi = __toESM(require_faSushi());
var export_faSushi = import_faSushi.definition;
export {
  export_faSushi as faSushi
};

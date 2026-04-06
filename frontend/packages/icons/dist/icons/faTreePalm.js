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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTreePalm.js
var require_faTreePalm = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTreePalm.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "tree-palm";
    var width = 576;
    var height = 512;
    var aliases = [127796];
    var unicode = "f82b";
    var svgPathData = "M102.9 91.4l30.3 50.5c-4 3.5-8 7.2-12 11.2c-71 71-60.2 151-32.7 191.3c5 7.3 15.2 7.3 21.4 1L248.4 206.9c4 10.8 7.8 26.2 9.7 47.2c4.2 46.5-1.6 116.8-32.7 216.4c-3 9.7-1.3 20.3 4.8 28.5s15.6 13 25.8 13l64 0c16.1 0 29.8-12 31.8-28c12.4-99-2.3-207.2-31.7-292l126.6 0c2.8 0 5.4-1.5 6.9-3.9l19.6-32.7c3.1-5.2 10.6-5.2 13.7 0l19.6 32.7c1.4 2.4 4 3.9 6.9 3.9l46.7 0c8.8 0 16.1-7.2 14.4-15.9C565.3 128.2 516.4 64 416 64c-43.7 0-77.6 12.2-102.8 29.8C296.5 49.4 248 0 160 0C59.6 0 10.7 64.2 1.6 112.1C-.1 120.8 7.2 128 16 128l46.7 0c2.8 0 5.4-1.5 6.9-3.9L89.1 91.4c3.1-5.2 10.6-5.2 13.7 0z";
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
    exports.faTreePalm = exports.definition;
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

// src/_entries/faTreePalm.js
var import_faTreePalm = __toESM(require_faTreePalm());
var export_faTreePalm = import_faTreePalm.definition;
export {
  export_faTreePalm as faTreePalm
};

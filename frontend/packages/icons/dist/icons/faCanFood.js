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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCanFood.js
var require_faCanFood = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCanFood.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "can-food";
    var width = 384;
    var height = 512;
    var aliases = [129387];
    var unicode = "e3e6";
    var svgPathData = "M270.3 304.5c12.1-2.5 23.4-5.5 33.7-9L304 426c-2.8 1.6-6.6 3.6-11.8 5.7C269.8 440.9 234.4 448 192 448s-77.8-7.1-100.2-16.3c-5.2-2.1-9-4.1-11.8-5.7l0-130.4c10.3 3.5 21.6 6.5 33.7 9c7.6 36.3 39.8 63.5 78.3 63.5s70.7-27.2 78.3-63.5zm-100.1-40a32 32 0 1 1 43.5 46.9 32 32 0 1 1 -43.5-46.9zM16 72l0 168 0 200c0 39.8 78.8 72 176 72s176-32.2 176-72l0-200 0-168c0-39.8-78.8-72-176-72S16 32.2 16 72zm288 0c0 13.3-50.1 24-112 24S80 85.3 80 72s50.1-24 112-24s112 10.7 112 24z";
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
    exports.faCanFood = exports.definition;
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

// src/_entries/faCanFood.js
var import_faCanFood = __toESM(require_faCanFood());
var export_faCanFood = import_faCanFood.definition;
export {
  export_faCanFood as faCanFood
};

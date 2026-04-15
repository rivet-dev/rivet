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

// src/node_modules/@fortawesome/free-brands-svg-icons/faVk.js
var require_faVk = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faVk.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "vk";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f189";
    var svgPathData = "M31.5 63.5C0 95 0 145.7 0 247L0 265C0 366.3 0 417 31.5 448.5S113.7 480 215 480l17.9 0c101.4 0 152.1 0 183.5-31.5S448 366.3 448 265l0-17.9c0-101.4 0-152.1-31.5-183.5S334.3 32 233 32L215 32C113.7 32 63 32 31.5 63.5zM75.6 168.3l51.1 0c1.7 85.5 39.4 121.7 69.3 129.2l0-129.2 48.2 0 0 73.7c29.5-3.2 60.5-36.8 70.9-73.7l48.2 0c-3.9 19.2-11.8 37.3-23.1 53.3s-25.7 29.5-42.5 39.6c18.7 9.3 35.2 22.4 48.4 38.5s22.9 34.9 28.3 55l-53 0c-4.9-17.5-14.8-33.1-28.6-45s-30.7-19.4-48.7-21.6l0 66.6-5.8 0c-102.1 0-160.3-70-162.8-186.5z";
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
    exports.faVk = exports.definition;
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

// src/_entries/faVk.js
var import_faVk = __toESM(require_faVk());
var export_faVk = import_faVk.definition;
export {
  export_faVk as faVk
};

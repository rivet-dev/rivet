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

// src/node_modules/@fortawesome/free-brands-svg-icons/faMedrt.js
var require_faMedrt = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faMedrt.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "medrt";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f3c8";
    var svgPathData = "M129.7 256c0 121.8 83.9 222.8 193.5 241.1-18.7 4.5-38.2 6.9-58.2 6.9-137.6 0-249-111-249-248S127.4 8 264.9 8c20.1 0 39.6 2.4 58.2 6.9-109.6 18.3-193.4 119.3-193.4 241.1zM427.1 356.3c-77.7 55.4-179.6 47.5-240.4-14.6 5.5 14.1 12.7 27.7 21.7 40.5 61.6 88.2 182.4 109.3 269.7 47s108.1-184.3 46.5-272.6c-9-12.9-19.3-24.3-30.5-34.2 37.4 78.8 10.7 178.5-67 233.9zm-218.8-244c-1.4 1-2.7 2.1-4 3.1 64.3-17.8 135.9 4 178.9 60.5 35.7 47 42.9 106.6 24.4 158 56.7-56.2 67.6-142.1 22.3-201.8-50-65.5-149.1-74.4-221.6-19.8zM312 224c-4.4 0-8-3.6-8-8l0-40c0-4.4-3.6-8-8-8l-48 0c-4.4 0-8 3.6-8 8l0 40c0 4.4-3.6 8-8 8l-40 0c-4.4 0-8 3.6-8 8l0 48c0 4.4 3.6 8 8 8l40 0c4.4 0 8 3.6 8 8l0 40c0 4.4 3.6 8 8 8l48 0c4.4 0 8-3.6 8-8l0-40c0-4.4 3.6-8 8-8l40 0c4.4 0 8-3.6 8-8l0-48c0-4.4-3.6-8-8-8l-40 0z";
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
    exports.faMedrt = exports.definition;
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

// src/_entries/faMedrt.js
var import_faMedrt = __toESM(require_faMedrt());
var export_faMedrt = import_faMedrt.definition;
export {
  export_faMedrt as faMedrt
};

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

// src/node_modules/@fortawesome/free-brands-svg-icons/faJxl.js
var require_faJxl = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faJxl.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "jxl";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e67b";
    var svgPathData = "M412.2 32L35.8 32C16 32 0 48 0 67.8L0 444.2C0 464 16 480 35.8 480l376.4 0c19.8 0 35.8-16 35.8-35.8l0-376.4C448 48 432 32 412.2 32zM378.6 333.7c0 40.2-32.6 72.8-72.8 72.8l-235.6 0c0-40.2 32.6-72.8 72.8-72.8l235.6 0zm0-113.9c0 40.2-32.6 72.8-72.8 72.8l-235.6 0c0-40.2 32.6-72.8 72.8-72.8l235.6 0zm0-113.9c0 40.2-32.6 72.8-72.8 72.8l-235.6 0c0-40.2 32.6-72.8 72.8-72.8l235.6 0z";
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
    exports.faJxl = exports.definition;
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

// src/_entries/faJxl.js
var import_faJxl = __toESM(require_faJxl());
var export_faJxl = import_faJxl.definition;
export {
  export_faJxl as faJxl
};

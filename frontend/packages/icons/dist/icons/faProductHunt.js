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

// src/node_modules/@fortawesome/free-brands-svg-icons/faProductHunt.js
var require_faProductHunt = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faProductHunt.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "product-hunt";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f288";
    var svgPathData = "M326.3 218.8c0 20.5-16.7 37.2-37.2 37.2l-70.3 0 0-74.4 70.3 0c20.5 0 37.2 16.7 37.2 37.2zM8 256a248 248 0 1 1 496 0 248 248 0 1 1 -496 0zm367.9-37.2c0-47.9-38.9-86.8-86.8-86.8l-119.9 0 0 248 49.6 0 0-74.4 70.3 0c47.9 0 86.8-38.9 86.8-86.8z";
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
    exports.faProductHunt = exports.definition;
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

// src/_entries/faProductHunt.js
var import_faProductHunt = __toESM(require_faProductHunt());
var export_faProductHunt = import_faProductHunt.definition;
export {
  export_faProductHunt as faProductHunt
};

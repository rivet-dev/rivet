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

// src/node_modules/@fortawesome/free-brands-svg-icons/faUber.js
var require_faUber = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faUber.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "uber";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f402";
    var svgPathData = "M414.1 32L33.9 32C15.2 32 0 47.2 0 65.9L0 446c0 18.8 15.2 34 33.9 34L414 480c18.7 0 33.9-15.2 33.9-33.9l0-380.2C448 47.2 432.8 32 414.1 32zM237.6 391.1C163 398.6 96.4 344.2 88.9 269.6l94.4 0 0 20.4c0 3.7 3 6.8 6.8 6.8l67.9 0c3.7 0 6.8-3 6.8-6.8l0-67.9c0-3.7-3-6.8-6.8-6.8l-67.9 0c-3.7 0-6.8 3-6.8 6.8l0 20.4-94.4 0c7-69.4 65.4-122.2 135.1-122.2s128.1 52.8 135.1 122.2c7.5 74.5-46.9 141.1-121.5 148.6z";
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
    exports.faUber = exports.definition;
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

// src/_entries/faUber.js
var import_faUber = __toESM(require_faUber());
var export_faUber = import_faUber.definition;
export {
  export_faUber as faUber
};

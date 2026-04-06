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

// src/node_modules/@fortawesome/free-solid-svg-icons/faShoePrints.js
var require_faShoePrints = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faShoePrints.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "shoe-prints";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f54b";
    var svgPathData = "M296 192c-21.1-12.1-42.3-24.2-72-29.3l0-140.3C257.7 13 311.4 0 352 0 448 0 576 48 576 128s-119.6 96-176 96c-48 0-76-16-104-32zM128 32l48 0 0 128-48 0c-35.3 0-64-28.7-64-64s28.7-64 64-64zM232 320c28-16 56-32 104-32 56.4 0 176 16 176 96S384 512 288 512c-40.5 0-94.3-13-128-22.4l0-140.3c29.7-5.2 50.9-17.3 72-29.4zM64 480c-35.3 0-64-28.7-64-64s28.7-64 64-64l48 0 0 128-48 0z";
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
    exports.faShoePrints = exports.definition;
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

// src/_entries/faShoePrints.js
var import_faShoePrints = __toESM(require_faShoePrints());
var export_faShoePrints = import_faShoePrints.definition;
export {
  export_faShoePrints as faShoePrints
};

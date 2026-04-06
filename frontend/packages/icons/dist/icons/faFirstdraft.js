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

// src/node_modules/@fortawesome/free-brands-svg-icons/faFirstdraft.js
var require_faFirstdraft = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faFirstdraft.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "firstdraft";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "f3a1";
    var svgPathData = "M384 192l-64 0 0 128-128 0 0 128-192 0 0-25.6 166.4 0 0-128 128 0 0-128 89.6 0 0 25.6zm-25.6 38.4l0 128-128 0 0 128-166.4 0 0 25.6 192 0 0-128 128 0 0-153.6-25.6 0zm25.6 192l-89.6 0 0 89.6 25.6 0 0-64 64 0 0-25.6zM0 0l0 384 128 0 0-128 128 0 0-128 128 0 0-128-384 0z";
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
    exports.faFirstdraft = exports.definition;
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

// src/_entries/faFirstdraft.js
var import_faFirstdraft = __toESM(require_faFirstdraft());
var export_faFirstdraft = import_faFirstdraft.definition;
export {
  export_faFirstdraft as faFirstdraft
};

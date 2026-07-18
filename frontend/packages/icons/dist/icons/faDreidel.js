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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faDreidel.js
var require_faDreidel = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faDreidel.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "dreidel";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f792";
    var svgPathData = "M438.6 86.6c12.5-12.5 12.5-32.8 0-45.3s-32.8-12.5-45.3 0L288 146.7 225.9 84.7c-18.7-18.7-49.1-18.7-67.9 0l-57.4 57.4L337.9 379.3l57.4-57.4c18.7-18.7 18.7-49.1 0-67.9L333.3 192 438.6 86.6zM315.3 401.9L78.1 164.7 18.7 224C6.7 236 0 252.3 0 269.3L0 416c0 35.3 28.7 64 64 64l146.7 0c17 0 33.3-6.7 45.3-18.7l59.3-59.3z";
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
    exports.faDreidel = exports.definition;
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

// src/_entries/faDreidel.js
var import_faDreidel = __toESM(require_faDreidel());
var export_faDreidel = import_faDreidel.definition;
export {
  export_faDreidel as faDreidel
};

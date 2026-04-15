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

// src/node_modules/@fortawesome/free-brands-svg-icons/faBimobject.js
var require_faBimobject = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faBimobject.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "bimobject";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f378";
    var svgPathData = "M416 32L32 32C14.4 32 0 46.4 0 64L0 448c0 17.6 14.4 32 32 32l384 0c17.6 0 32-14.4 32-32l0-384c0-17.6-14.4-32-32-32zM352 289.4c0 49.4-11.4 82.6-103.8 82.6l-16.9 0c-44.1 0-62.4-14.9-70.4-38.8l-.9 0 0 34.8-64 0 0-232 64 0 0 74.7 1.1 0c4.6-30.5 39.7-38.8 69.7-38.8l17.3 0c92.4 0 103.8 33.1 103.8 82.5l0 35 .1 0zm-64-28.9l0 22.9c0 21.7-3.4 33.8-38.4 33.8l-45.3 0c-28.9 0-44.1-6.5-44.1-35.7l0-19c0-29.3 15.2-35.7 44.1-35.7l45.3 0c35-.2 38.4 12 38.4 33.7z";
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
    exports.faBimobject = exports.definition;
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

// src/_entries/faBimobject.js
var import_faBimobject = __toESM(require_faBimobject());
var export_faBimobject = import_faBimobject.definition;
export {
  export_faBimobject as faBimobject
};

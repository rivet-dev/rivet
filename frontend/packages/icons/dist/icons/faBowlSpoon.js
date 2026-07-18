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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBowlSpoon.js
var require_faBowlSpoon = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBowlSpoon.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bowl-spoon";
    var width = 512;
    var height = 512;
    var aliases = [129379];
    var unicode = "e3e0";
    var svgPathData = "M112 192C50.1 192 0 149 0 96S50.1 0 112 0c48.8 0 90.3 26.7 105.6 64L480 64c17.7 0 32 14.3 32 32s-14.3 32-32 32l-262.4 0c-15.4 37.3-56.9 64-105.6 64zM2 257.5C.9 239.3 15.4 224 33.6 224l444.8 0c18.2 0 32.7 15.3 31.6 33.5c-5.4 91.7-59.7 169.7-136.4 209.7l-1.2 9.7c-2.5 20-19.5 35-39.7 35l-153.4 0c-20.2 0-37.2-15-39.7-35l-1.2-9.7C61.6 427.2 7.4 349.3 2 257.5z";
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
    exports.faBowlSpoon = exports.definition;
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

// src/_entries/faBowlSpoon.js
var import_faBowlSpoon = __toESM(require_faBowlSpoon());
var export_faBowlSpoon = import_faBowlSpoon.definition;
export {
  export_faBowlSpoon as faBowlSpoon
};

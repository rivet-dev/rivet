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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faAngle.js
var require_faAngle = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faAngle.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "angle";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e08c";
    var svgPathData = "M252.6 78.3c7.9-15.8 1.5-35-14.3-42.9s-35-1.5-42.9 14.3l-192 384c-5 9.9-4.4 21.7 1.4 31.1S20.9 480 32 480l384 0c17.7 0 32-14.3 32-32s-14.3-32-32-32L83.8 416 252.6 78.3zm-12.2 96l-21.9 43.8c21.4 17.4 40.4 37.7 56.2 60.4l40.8-25.3c-20.7-30.1-46.1-56.7-75.1-78.9zM321.1 384l49 0c-7-37-19.8-71.9-37.6-103.7l-40.8 25.3c13.3 24.2 23.3 50.5 29.5 78.3z";
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
    exports.faAngle = exports.definition;
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

// src/_entries/faAngle.js
var import_faAngle = __toESM(require_faAngle());
var export_faAngle = import_faAngle.definition;
export {
  export_faAngle as faAngle
};

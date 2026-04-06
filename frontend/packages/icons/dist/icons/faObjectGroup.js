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

// src/node_modules/@fortawesome/free-solid-svg-icons/faObjectGroup.js
var require_faObjectGroup = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faObjectGroup.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "object-group";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f247";
    var svgPathData = "M32 119.4C12.9 108.4 0 87.7 0 64 0 28.7 28.7 0 64 0 87.7 0 108.4 12.9 119.4 32l337.1 0c11.1-19.1 31.7-32 55.4-32 35.3 0 64 28.7 64 64 0 23.7-12.9 44.4-32 55.4l0 273.1c19.1 11.1 32 31.7 32 55.4 0 35.3-28.7 64-64 64-23.7 0-44.4-12.9-55.4-32l-337.1 0c-11.1 19.1-31.7 32-55.4 32-35.3 0-64-28.7-64-64 0-23.7 12.9-44.4 32-55.4l0-273.1zm448 0c-9.7-5.6-17.8-13.7-23.4-23.4L119.4 96c-5.6 9.7-13.7 17.8-23.4 23.4l0 273.1c9.7 5.6 17.8 13.7 23.4 23.4l337.1 0c5.6-9.7 13.7-17.8 23.4-23.4l0-273.1zM144 176c0-17.7 14.3-32 32-32l112 0c17.7 0 32 14.3 32 32l0 64c0 17.7-14.3 32-32 32l-112 0c-17.7 0-32-14.3-32-32l0-64zM256 320l32 0c44.2 0 80-35.8 80-80l32 0c17.7 0 32 14.3 32 32l0 64c0 17.7-14.3 32-32 32l-112 0c-17.7 0-32-14.3-32-32l0-16z";
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
    exports.faObjectGroup = exports.definition;
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

// src/_entries/faObjectGroup.js
var import_faObjectGroup = __toESM(require_faObjectGroup());
var export_faObjectGroup = import_faObjectGroup.definition;
export {
  export_faObjectGroup as faObjectGroup
};

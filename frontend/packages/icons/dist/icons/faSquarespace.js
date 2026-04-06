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

// src/node_modules/@fortawesome/free-brands-svg-icons/faSquarespace.js
var require_faSquarespace = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faSquarespace.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "squarespace";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f5be";
    var svgPathData = "M186.1 343.3c-9.7 9.7-9.7 25.3 0 34.9s25.3 9.6 34.9 0L378.3 221.1c19.3-19.3 50.6-19.3 69.9 0s19.3 50.6 0 69.9L294 445.1c19.3 19.3 50.5 19.3 69.8 0l0 0 119.3-119.2c38.6-38.6 38.6-101.1 0-139.7-38.6-38.6-101.2-38.6-139.7 0L186.1 343.3zM430.7 238.5c-9.7-9.7-25.3-9.7-34.9 0L238.5 395.7c-19.3 19.3-50.5 19.3-69.8 0l0 0c-9.6-9.6-25.3-9.7-34.9 0l0 0c-9.7 9.6-9.7 25.3 0 34.9l0 0c38.6 38.6 101.1 38.6 139.7 0L430.7 273.5c9.6-9.7 9.6-25.3 0-34.9zm-262 87.3L325.9 168.7c9.6-9.6 9.6-25.3 0-34.9-9.6-9.6-25.3-9.6-34.9 0L133.7 290.9c-19.3 19.3-50.6 19.3-69.9 0l0 0c-19.3-19.3-19.3-50.5 0-69.8l0 0 154.2-154.2c-19.3-19.3-50.5-19.3-69.8 0l0 0-119.2 119.3c-38.6 38.6-38.6 101.1 0 139.7 38.6 38.6 101.1 38.6 139.7 0zM81.3 273.5c9.6 9.6 25.3 9.6 34.9 0L273.5 116.3c19.3-19.3 50.6-19.3 69.8 0l0 0c9.7 9.6 25.3 9.6 34.9 0s9.6-25.3 0-34.9c-38.6-38.6-101.1-38.6-139.7 0L81.3 238.5c-9.6 9.6-9.6 25.3 0 34.9l0 0z";
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
    exports.faSquarespace = exports.definition;
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

// src/_entries/faSquarespace.js
var import_faSquarespace = __toESM(require_faSquarespace());
var export_faSquarespace = import_faSquarespace.definition;
export {
  export_faSquarespace as faSquarespace
};

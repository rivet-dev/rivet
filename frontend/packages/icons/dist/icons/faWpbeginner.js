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

// src/node_modules/@fortawesome/free-brands-svg-icons/faWpbeginner.js
var require_faWpbeginner = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faWpbeginner.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "wpbeginner";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f297";
    var svgPathData = "M463.2 322.4c56.2 64.3 4.2 157.6-91.9 157.6-39.6 0-78.8-17.7-100.1-50-6.9 .4-22.7 .4-29.6 0-21.4 32.4-60.6 50-100.1 50-95.5 0-148.3-93-91.9-157.6-79.1-131.9 31.3-290.4 206.8-290.4 175.6 0 285.9 158.6 206.8 290.4zm-339.6-83l41.5 0 0-58.1-41.5 0 0 58.1zm217.2 86.1l0-23.8c-60.5 20.9-132.4 9.2-187.6-34l.2 24.9c51.1 46.4 131.7 57.9 187.3 32.9zM190 239.4l166.1 0 0-58.1-166.1 0 0 58.1z";
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
    exports.faWpbeginner = exports.definition;
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

// src/_entries/faWpbeginner.js
var import_faWpbeginner = __toESM(require_faWpbeginner());
var export_faWpbeginner = import_faWpbeginner.definition;
export {
  export_faWpbeginner as faWpbeginner
};

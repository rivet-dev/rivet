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

// src/node_modules/@fortawesome/free-brands-svg-icons/faYoast.js
var require_faYoast = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faYoast.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "yoast";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f2b1";
    var svgPathData = "M91.3 76l186 0-7 18.9-179 0c-39.7 0-71.9 31.6-71.9 70.3l0 205.4c0 35.4 24.9 70.3 84 70.3l0 19.1-12.1 0C41.2 460 0 419.8 0 370.5L0 165.2C0 115.9 40.7 76 91.3 76zM320.4 20l66.5 0c-143.8 378.1-145.7 398.9-184.7 439.3-20.8 21.6-49.3 31.7-78.3 32.7l0-51.1c49.2-7.7 64.6-49.9 64.6-75.3 0-20.1 .6-12.6-82.1-223.2l61.4 0 50.4 156.6 102.2-279zM448 161.5l0 298.5-214 0c6.6-9.6 10.7-16.3 12.1-19.4l182.5 0 0-279.1c0-32.5-17.1-51.9-48.2-62.9L387.1 81c41.7 13.6 60.9 43.1 60.9 80.5z";
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
    exports.faYoast = exports.definition;
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

// src/_entries/faYoast.js
var import_faYoast = __toESM(require_faYoast());
var export_faYoast = import_faYoast.definition;
export {
  export_faYoast as faYoast
};

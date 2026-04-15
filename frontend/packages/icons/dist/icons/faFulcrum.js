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

// src/node_modules/@fortawesome/free-brands-svg-icons/faFulcrum.js
var require_faFulcrum = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faFulcrum.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "fulcrum";
    var width = 256;
    var height = 512;
    var aliases = [];
    var unicode = "f50b";
    var svgPathData = "M63.8 164.1L28.4 207.7-7 164.1 28.4 120.6 63.8 164.1zM112.2 0l-20.5 198.2-51 57.8 51 57.8 20.5 198.2 0-211.1-41.1-44.9 41.1-44.9 0-211.1zm79.7 164.1l35.4 43.6 35.4-43.6-35.4-43.5-35.4 43.5zm-48.5 47l41.1 44.9-41.1 44.9 0 211.1 20.6-198.2 51-57.8-51-57.8-20.6-198.2 0 211.1z";
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
    exports.faFulcrum = exports.definition;
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

// src/_entries/faFulcrum.js
var import_faFulcrum = __toESM(require_faFulcrum());
var export_faFulcrum = import_faFulcrum.definition;
export {
  export_faFulcrum as faFulcrum
};

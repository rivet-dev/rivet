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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHexagonImage.js
var require_faHexagonImage = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHexagonImage.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hexagon-image";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e504";
    var svgPathData = "M17.1 220c-12.9 22.3-12.9 49.7 0 72l88.3 152.9c12.9 22.3 36.6 36 62.4 36l176.6 0c25.7 0 49.5-13.7 62.4-36L494.9 292c12.9-22.3 12.9-49.7 0-72L406.6 67.1c-12.9-22.3-36.6-36-62.4-36l-176.6 0c-25.7 0-49.5 13.7-62.4 36L17.1 220zm141.2 59.8c4.5-4.9 10.8-7.7 17.4-7.8s13 2.6 17.6 7.4L232.2 320l71.6-86.2c4.6-5.5 11.3-8.7 18.5-8.7s13.9 3.2 18.5 8.7l72.6 87.4-55.2 95.7c-2.9 5-8.1 8-13.9 8l-176.6 0c-5.7 0-11-3-13.9-8l-46.8-81 51.2-56.1zM192 120a40 40 0 1 1 0 80 40 40 0 1 1 0-80z";
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
    exports.faHexagonImage = exports.definition;
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

// src/_entries/faHexagonImage.js
var import_faHexagonImage = __toESM(require_faHexagonImage());
var export_faHexagonImage = import_faHexagonImage.definition;
export {
  export_faHexagonImage as faHexagonImage
};

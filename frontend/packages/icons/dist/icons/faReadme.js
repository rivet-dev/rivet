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

// src/node_modules/@fortawesome/free-brands-svg-icons/faReadme.js
var require_faReadme = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faReadme.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "readme";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f4d5";
    var svgPathData = "M528.3 46.5l-139.8 0c-48.1 0-89.9 33.3-100.4 80.3-10.6-47-52.3-80.3-100.4-80.3L48 46.5C21.5 46.5 0 68 0 94.5L0 340.3c0 26.5 21.5 48 48 48l89.7 0c102.2 0 132.7 24.4 147.3 75 .7 2.8 5.2 2.8 6 0 14.7-50.6 45.2-75 147.3-75l89.7 0c26.5 0 48-21.5 48-48l0-245.7c0-26.4-21.3-47.9-47.7-48.1zM242 311.9c0 1.9-1.5 3.5-3.5 3.5l-160.3 0c-1.9 0-3.5-1.5-3.5-3.5l0-22.9c0-1.9 1.5-3.5 3.5-3.5l160.4 0c1.9 0 3.5 1.5 3.5 3.5l0 22.9-.1 0zm0-60.9c0 1.9-1.5 3.5-3.5 3.5l-160.3 0c-1.9 0-3.5-1.5-3.5-3.5l0-22.9c0-1.9 1.5-3.5 3.5-3.5l160.4 0c1.9 0 3.5 1.5 3.5 3.5l0 22.9-.1 0zm0-60.9c0 1.9-1.5 3.5-3.5 3.5l-160.3 0c-1.9 0-3.5-1.5-3.5-3.5l0-22.9c0-1.9 1.5-3.5 3.5-3.5l160.4 0c1.9 0 3.5 1.5 3.5 3.5l0 22.9-.1 0zM501.3 311.8c0 1.9-1.5 3.5-3.5 3.5l-160.3 0c-1.9 0-3.5-1.5-3.5-3.5l0-22.9c0-1.9 1.5-3.5 3.5-3.5l160.4 0c1.9 0 3.5 1.5 3.5 3.5l0 22.9-.1 0zm0-60.9c0 1.9-1.5 3.5-3.5 3.5l-160.3 0c-1.9 0-3.5-1.5-3.5-3.5l0-22.9c0-1.9 1.5-3.5 3.5-3.5l160.4 0c1.9 0 3.5 1.5 3.5 3.5l0 22.9-.1 0zm0-60.9c0 1.9-1.5 3.5-3.5 3.5l-160.3 0c-1.9 0-3.5-1.5-3.5-3.5l0-22.8c0-1.9 1.5-3.5 3.5-3.5l160.4 0c1.9 0 3.5 1.5 3.5 3.5l0 22.8-.1 0z";
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
    exports.faReadme = exports.definition;
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

// src/_entries/faReadme.js
var import_faReadme = __toESM(require_faReadme());
var export_faReadme = import_faReadme.definition;
export {
  export_faReadme as faReadme
};

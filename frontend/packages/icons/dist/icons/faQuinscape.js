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

// src/node_modules/@fortawesome/free-brands-svg-icons/faQuinscape.js
var require_faQuinscape = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faQuinscape.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "quinscape";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f459";
    var svgPathData = "M313.7 474.6l-1 0c-41.9 0-82.1-16.7-111.8-46.3s-46.3-69.9-46.3-111.8 16.7-82.1 46.3-111.8 69.9-46.3 111.8-46.3c94.9 0 168.2 83.1 157 176.6 4 5.1 8.2 9.6 11.2 15.3 13.4-30.3 20.3-62.4 20.3-97.7 0-135.1-109.5-244.6-244.6-244.6S12.1 117.5 12.1 252.6 121.6 497.2 256.6 497.2c23.8 .2 47.6-3.2 70.4-10.1-5.2-3.5-8.9-8.1-13.3-12.5zm78.3-169a99.2 99.2 0 1 0 0 198.4 99.2 99.2 0 1 0 0-198.4z";
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
    exports.faQuinscape = exports.definition;
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

// src/_entries/faQuinscape.js
var import_faQuinscape = __toESM(require_faQuinscape());
var export_faQuinscape = import_faQuinscape.definition;
export {
  export_faQuinscape as faQuinscape
};

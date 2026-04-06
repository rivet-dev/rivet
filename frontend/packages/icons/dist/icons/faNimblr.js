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

// src/node_modules/@fortawesome/free-brands-svg-icons/faNimblr.js
var require_faNimblr = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faNimblr.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "nimblr";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "f5a8";
    var svgPathData = "M246.6 353.3a27 27 0 1 1 0-54 27 27 0 1 1 0 54zm-79.4-27a27.1 27.1 0 1 1 -54.3 0 27.1 27.1 0 1 1 54.3 0zM191.8 159C157 159 89.4 178.8 59.2 227L14 0 14 335.5C14 433.1 93.6 512 191.8 512S369.5 433 369.5 335.5 290.1 159 191.8 159zm0 308.1c-73.3 0-132.5-58.9-132.5-131.6s59.2-131.6 132.5-131.6 132.5 58.9 132.5 131.5-59.3 131.6-132.5 131.6l0 .1z";
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
    exports.faNimblr = exports.definition;
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

// src/_entries/faNimblr.js
var import_faNimblr = __toESM(require_faNimblr());
var export_faNimblr = import_faNimblr.definition;
export {
  export_faNimblr as faNimblr
};

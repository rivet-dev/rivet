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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faStarSharpHalf.js
var require_faStarSharpHalf = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faStarSharpHalf.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "star-sharp-half";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e28c";
    var svgPathData = "M254.4 50.4L214.5 176 90.3 176 88 176l-13.3 0L56 176c-10.3 0-19.4 6.5-22.7 16.2s-.1 20.4 8 26.7L56 230.4l10.5 8.2 1.8 1.4 103.7 80.7L132 446.7l-1.1 3.5L126.1 465l-5 15.7c-3.2 10 .5 21 9.1 27s20.2 5.7 28.5-.7l13-10.1 12.3-9.5 2.9-2.3L288 406.4l0-81.1 0-169.4L288 0c-10.5 0-19.7 6.8-22.9 16.7L260 32.9s0 0 0 0l-4.6 14.3-1 3.2z";
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
    exports.faStarSharpHalf = exports.definition;
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

// src/_entries/faStarSharpHalf.js
var import_faStarSharpHalf = __toESM(require_faStarSharpHalf());
var export_faStarSharpHalf = import_faStarSharpHalf.definition;
export {
  export_faStarSharpHalf as faStarSharpHalf
};

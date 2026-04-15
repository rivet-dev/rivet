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

// src/node_modules/@fortawesome/free-solid-svg-icons/faFeatherPointed.js
var require_faFeatherPointed = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faFeatherPointed.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "feather-pointed";
    var width = 512;
    var height = 512;
    var aliases = ["feather-alt"];
    var unicode = "f56b";
    var svgPathData = "M475.3 .1c9.9-.8 19.6 3 26.6 10s10.8 16.7 10 26.6c-4 49.3-17.4 126.2-46.3 199.7-1.8 4.5-5.5 7.9-10.2 9.3L374.5 270c-3.9 1.2-6.5 4.7-6.5 8.8 0 5.1 4.1 9.2 9.2 9.2l38.6 0c12 0 19.7 12.8 13.5 23.1-4 6.7-8.3 13.2-12.7 19.6-2 2.9-5 5-8.4 6.1L310.5 366c-3.9 1.2-6.5 4.7-6.5 8.8 0 5.1 4.1 9.2 9.2 9.2l16 0c14.6 0 21 17.4 8.8 25.4-68 45-137.7 43.3-182.4 31.3-12.7-3.4-24-9.9-34.4-17.9L48 496c-8.8 8.8-23.2 8.8-32 0s-8.8-23.2 0-32l80-80 .5 .5c.7-1.3 1.6-2.5 2.7-3.6L256 224c8.8-8.8 8.8-23.2 0-32s-23.2-8.8-32 0L89.7 326.2c-8.9 8.9-24 4.4-25-8.2-4.3-53.2 9.3-123.1 72.6-186.4 91.1-91.1 254.2-124.7 337.9-131.5z";
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
    exports.faFeatherPointed = exports.definition;
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

// src/_entries/faFeatherPointed.js
var import_faFeatherPointed = __toESM(require_faFeatherPointed());
var export_faFeatherPointed = import_faFeatherPointed.definition;
export {
  export_faFeatherPointed as faFeatherPointed
};

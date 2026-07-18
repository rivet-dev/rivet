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

// src/node_modules/@fortawesome/free-brands-svg-icons/faMicroblog.js
var require_faMicroblog = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faMicroblog.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "microblog";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e01a";
    var svgPathData = "M399.8 362.2c29.5-34.7 47.1-78.3 47.1-125.8 0-113-99.6-204.4-222.5-204.4S2 123.5 2 236.4 101.6 440.9 224.5 440.9c27 0 53.9-4.5 79.4-13.4 1.4-.5 3-.5 4.5-.1s2.7 1.4 3.6 2.6c18.6 25.1 47.6 42.7 79.9 49.9 1.1 .2 2.3 0 3.3-.6s1.7-1.6 1.9-2.8c.1-.6 .1-1.3 0-1.9s-.4-1.2-.8-1.7c-12.3-16-18.7-35.8-18-56s8.4-39.5 21.7-54.7l-.2 .1zM330 212.4l-57.3 43.5 20.8 68.9c.4 1.3 .4 2.7-.1 4s-1.2 2.4-2.3 3.2-2.4 1.2-3.8 1.2-2.7-.4-3.8-1.2l-59.1-41-59.1 41.1c-1.1 .8-2.4 1.2-3.8 1.2s-2.7-.4-3.8-1.2-1.9-1.9-2.3-3.2-.5-2.7-.1-4l20.8-68.9-57.3-43.5c-1.1-.8-1.9-1.9-2.3-3.2s-.4-2.7 0-4 1.2-2.4 2.3-3.2 2.4-1.3 3.7-1.3l71.9-1.5 23.7-67.9c.4-1.3 1.3-2.4 2.4-3.2s2.4-1.2 3.8-1.2 2.7 .4 3.8 1.2 1.9 1.9 2.4 3.2l23.7 67.9 71.9 1.5c1.4 0 2.7 .4 3.8 1.2s1.9 1.9 2.3 3.2 .4 2.7 0 4-1.2 2.4-2.3 3.3l0 0z";
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
    exports.faMicroblog = exports.definition;
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

// src/_entries/faMicroblog.js
var import_faMicroblog = __toESM(require_faMicroblog());
var export_faMicroblog = import_faMicroblog.definition;
export {
  export_faMicroblog as faMicroblog
};

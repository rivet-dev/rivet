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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGramophone.js
var require_faGramophone = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGramophone.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "gramophone";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "f8bd";
    var svgPathData = "M127 0c10.5-.4 20 6 23.6 15.9c0 0 0 .1 0 .1l.3 .7c.3 .7 .7 1.7 1.3 3.2c1.2 2.9 3.2 7.3 6 12.8c5.7 11.1 14.8 27 28.2 45.1c26.9 36.2 71.4 81.5 141.3 116.8c17.3 8.7 31.6 22.9 40.1 40.4c5.4 10.9 8.2 23 8.2 35l0 .3 0 .3c0 45-36.5 81.4-81.4 81.4L184 352c-13.3 0-24-10.7-24-24s10.7-24 24-24l110 0c18.7 0 33.9-15.1 34-33.7c-.1-5.4-1.5-10.7-4.1-15.6c-3.1-4.9-7.7-7.8-12.4-8.6c-44.1-7.3-133.1-7.9-241.4 69.4c-.3 .2-.7 .5-1 .7c-2.7 1.8-5.6 2.9-8.7 3.5c-3.2 .6-6.6 .6-10-.2c-1.2-.3-2.4-.7-3.6-1.1c-.5-.2-1-.5-1.5-.7c-.9-.4-1.7-.9-2.5-1.5c-1.2-.8-2.2-1.6-3.2-2.6c-1.4-1.3-2.5-2.7-3.6-4.2c-1.8-2.8-3-5.8-3.6-8.9c-.6-3.1-.5-6.4 .2-9.6c.1-.3 .2-.7 .3-1L104.8 17.9C107.5 7.7 116.5 .4 127 0zM32 416c0-17.7 14.3-32 32-32l256 0c17.7 0 32 14.3 32 32l0 32c17.7 0 32 14.3 32 32s-14.3 32-32 32L32 512c-17.7 0-32-14.3-32-32s14.3-32 32-32l0-32z";
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
    exports.faGramophone = exports.definition;
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

// src/_entries/faGramophone.js
var import_faGramophone = __toESM(require_faGramophone());
var export_faGramophone = import_faGramophone.definition;
export {
  export_faGramophone as faGramophone
};

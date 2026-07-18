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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSaxophone.js
var require_faSaxophone = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSaxophone.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "saxophone";
    var width = 640;
    var height = 512;
    var aliases = [127927];
    var unicode = "f8dc";
    var svgPathData = "M246.7 176L171.4 306.1c-2.2 3.8-3.4 8.2-3.4 12.6l0 2.6c0 12.5 10.1 22.6 22.6 22.6c6 0 11.8-2.4 16-6.6L319 225l-8-8c-9.4-9.4-9.4-24.6 0-33.9s24.6-9.4 33.9 0l8 8L367 177l-8-8c-9.4-9.4-9.4-24.6 0-33.9s24.6-9.4 33.9 0l8 8L415 129l-8-8c-9.4-9.4-9.4-24.6 0-33.9s24.6-9.4 33.9 0l8 8 44.3-44.3c12-12 28.3-18.7 45.3-18.7l30.3 0c15 0 29.5 5.2 41 14.8l24.3 20.3c3.7 3.1 5.9 7.7 5.9 12.6c0 9-7.3 16.3-16.3 16.3l-64.2 0c-9.8 0-19 4.5-25.1 12.1L288.6 418.6C257.9 457.4 211.2 480 161.8 480C72.4 480 0 407.6 0 318.2l0-3.1c0-12.7 1.5-25.4 4.4-37.8L28.2 176 24 176c-13.3 0-24-10.7-24-24s10.7-24 24-24l240 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-17.3 0zM120 320a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zm8-72a24 24 0 1 0 0-48 24 24 0 1 0 0 48z";
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
    exports.faSaxophone = exports.definition;
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

// src/_entries/faSaxophone.js
var import_faSaxophone = __toESM(require_faSaxophone());
var export_faSaxophone = import_faSaxophone.definition;
export {
  export_faSaxophone as faSaxophone
};

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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTamale.js
var require_faTamale = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTamale.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "tamale";
    var width = 320;
    var height = 512;
    var aliases = [129748];
    var unicode = "e451";
    var svgPathData = "M48.1 18.3C72.3 8.6 108.4 0 160 0s87.7 8.6 111.9 18.3c12.1 4.8 21 9.9 27.4 14.1c12.7 8.5 23.1 20.4 20.3 36.9L296.4 208l-.4 0H236.2L248 72.7c.4-4.4-2.9-8.3-7.3-8.7s-8.3 2.9-8.7 7.3L220.1 208H168V64c0-4.4-3.6-8-8-8s-8 3.6-8 8V208H99.9L88 71.3c-.4-4.4-4.3-7.7-8.7-7.3s-7.7 4.3-7.3 8.7L83.8 208H24l-.4 0L.4 69.3C-2.3 52.8 8.1 40.8 20.8 32.4c6.3-4.2 15.3-9.3 27.4-14.1zM24 240H296c8.8 0 16 7.2 16 16s-7.2 16-16 16H192l28.8 38.4c5.3 7.1 3.9 17.1-3.2 22.4s-17.1 3.9-22.4-3.2L160 282.7l-35.2 46.9c-5.3 7.1-15.3 8.5-22.4 3.2s-8.5-15.3-3.2-22.4L128 272H24c-8.8 0-16-7.2-16-16s7.2-16 16-16zM152 448c0 4.4 3.6 8 8 8s8-3.6 8-8V346.7c13.4 17.9 35.9 26.5 57.4 18.1l6.6 76c.4 4.4 4.3 7.7 8.7 7.3s7.7-4.3 7.3-8.7l-7.3-84.1c14.2-13.2 18.9-33.4 12.6-51.2H296l.4 0 23.1 138.7c1.7 10.2-1.6 20.6-8.9 27.9l0 0c-10.6 10.5-25.1 17.6-38.7 23C247.7 503.4 211.6 512 160 512s-87.7-8.6-111.9-18.3c-13.7-5.5-28.1-12.5-38.7-23l0 0c-7.3-7.3-10.6-17.7-8.9-27.9L23.6 304l.4 0H66.7c-6.3 17.7-1.6 38 12.6 51.2L72 439.3c-.4 4.4 2.9 8.3 7.3 8.7s8.3-2.9 8.7-7.3l6.6-76c21.5 8.4 44-.2 57.4-18.1V448z";
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
    exports.faTamale = exports.definition;
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

// src/_entries/faTamale.js
var import_faTamale = __toESM(require_faTamale());
var export_faTamale = import_faTamale.definition;
export {
  export_faTamale as faTamale
};
